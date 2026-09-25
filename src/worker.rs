//! Rollout worker process ??? collection.

use crate::algorithms::AlgorithmLayout;
use crate::games::EnvSpec;
use crate::logging::{EpisodeState, TrainingLogger};
use crate::param::RuntimeConfig;
use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3::types::{PyByteArray, PyDict, PyList};
use std::collections::VecDeque;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::thread::JoinHandle;
use std::time::Duration;
use tokio::sync::{oneshot, watch};

/*
    Main Entry Point of RL-rs.
    Training Workers of thrl.
    Copyright (C) 2026  T. Liu (touhourl@proton.me) and contributors of thrl project

    This program is free software: you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/

#[derive(Debug, Clone, Copy)]
struct ControlState {
    paused: bool,
    shutdown: bool,
    policy_version: usize,
    config_version: usize,
}

struct InferenceResponse {
    action: usize,
    aux: Vec<f32>,
    state: Vec<f32>,
}

struct InferenceRequest {
    policy_version: usize,
    config_version: usize,
    observations: Vec<Arc<Vec<f32>>>,
    state: Vec<f32>,
    forced_action: Option<usize>,
}

struct InferenceEnvelope {
    request: InferenceRequest,
    reply: oneshot::Sender<InferenceResponse>,
}

// Rollout Chunks from worker (resv) for PPO updates
struct RolloutChunk {
    worker_id: usize,
    policy_version: usize,
    config_version: usize,
    observations: Vec<Vec<f32>>,
    actions: Vec<i64>,
    rewards: Vec<f32>,
    dones: Vec<f32>,
    aux: Vec<f32>,
    state: Vec<f32>,
    tail_observations: Vec<Arc<Vec<f32>>>,
    tail_aux: Vec<f32>,
    len: usize,
    successes: usize,
    gameovers: usize,
}

struct RolloutBuffer {
    // We preallocate the trajectory buffers, so we do not need to refrequently do it in training loop.
    // Features, Maps, Discrete Actions, Rewards, Terminal States, Log Probabilities, Critic Predictions, Recurrent State
    // Note that, we are doing rust. So we use f32 so for a better calculation.
    observations: Vec<Vec<f32>>,
    actions: Vec<i64>,
    rewards: Vec<f32>,
    dones: Vec<f32>,
    aux: Vec<f32>,
    state: Vec<f32>,
    len: usize,
    last_done: bool,
    successes: usize,
    gameovers: usize,
}

fn take_reusable<T>(values: &mut Vec<T>) -> Vec<T> {
    let capacity = values.capacity();
    std::mem::replace(values, Vec::with_capacity(capacity))
}

impl RolloutBuffer {
    fn new(cfg: &RuntimeConfig, spec: &EnvSpec, layout: AlgorithmLayout) -> Self {
        let chunk_size = cfg.worker.chunk_size;
        Self {
            observations: spec
                .observations
                .iter()
                .map(|shape| Vec::with_capacity(chunk_size * shape.iter().product::<usize>()))
                .collect(),
            actions: Vec::with_capacity(chunk_size),
            rewards: Vec::with_capacity(chunk_size * spec.rewards),
            dones: Vec::with_capacity(chunk_size),
            aux: Vec::with_capacity(chunk_size * layout.aux_size),
            state: Vec::with_capacity(chunk_size * layout.state_size),
            len: 0,
            last_done: false,
            successes: 0,
            gameovers: 0,
        }
    }

    fn clear(&mut self) {
        self.observations.iter_mut().for_each(Vec::clear);
        self.actions.clear();
        self.rewards.clear();
        self.dones.clear();
        self.aux.clear();
        self.state.clear();
        self.len = 0;
        self.last_done = false;
        self.successes = 0;
        self.gameovers = 0;
    }

    fn take(
        &mut self,
        worker_id: usize,
        policy_version: usize,
        config_version: usize,
        tail_observations: Vec<Arc<Vec<f32>>>,
        tail_aux: Vec<f32>,
    ) -> RolloutChunk {
        let chunk = RolloutChunk {
            worker_id,
            policy_version,
            config_version,
            observations: self.observations.iter_mut().map(take_reusable).collect(),
            actions: take_reusable(&mut self.actions),
            rewards: take_reusable(&mut self.rewards),
            dones: take_reusable(&mut self.dones),
            aux: take_reusable(&mut self.aux),
            state: take_reusable(&mut self.state),
            tail_observations,
            tail_aux,
            len: self.len,
            successes: self.successes,
            gameovers: self.gameovers,
        };
        self.clear();
        chunk
    }
}

struct EpisodeCoordinator {
    episode: usize,
    cfg: String,
    logger: TrainingLogger,
}

impl EpisodeCoordinator {
    fn record(&mut self, rewards: &[f32], length: usize, flag: u8) {
        self.episode += 1;
        let final_state = match flag {
            2 => EpisodeState::Success,
            1 => EpisodeState::GameOver,
            _ => EpisodeState::Running,
        };
        if let Err(err) =
            self.logger
                .log_episode_json(self.episode, rewards, length, final_state, &self.cfg)
        {
            tracing::warn!("Failed to log episode: {}", err);
        }
    }
}
// 208 Implementations??? rust-analyzer, are you kidding me???
// by the way I am not a fan with Mutex and Arc.
#[pyclass(module = "rrr")]
pub struct Collector {
    cfg: RuntimeConfig,
    request_rx: Mutex<mpsc::Receiver<InferenceEnvelope>>,
    chunk_rx: Mutex<mpsc::Receiver<RolloutChunk>>,
    pending: Mutex<VecDeque<oneshot::Sender<InferenceResponse>>>,
    control_tx: watch::Sender<ControlState>,
    paused_workers: Arc<Vec<AtomicBool>>,
    worker_errors: Arc<Mutex<Vec<Option<String>>>>,
    threads: Vec<JoinHandle<()>>,
    episode: Arc<Mutex<EpisodeCoordinator>>,
    curriculum_program: String,
    env_spec: EnvSpec,
    layout: AlgorithmLayout,
}

#[pymethods]
impl Collector {
    #[new]
    #[pyo3(signature = (start_episode=0, start_update_step=0, cfg_json=None, state_size=0, aux_size=0))]
    pub fn new(
        start_episode: usize,
        start_update_step: usize,
        cfg_json: Option<&str>,
        state_size: usize,
        aux_size: usize,
    ) -> PyResult<Self> {
        let cfg = RuntimeConfig::global().clone();
        let env_spec = crate::games::spec(&cfg).map_err(PyRuntimeError::new_err)?;
        if cfg.reward.scales.len() != env_spec.rewards {
            return Err(PyRuntimeError::new_err(format!(
                "Reward scale count {} does not match environment reward count {}",
                cfg.reward.scales.len(),
                env_spec.rewards
            )));
        }
        let layout = AlgorithmLayout {
            state_size,
            aux_size,
        };
        let curriculum_program = std::fs::read_to_string(&cfg.paths.curriculum_file)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
        let curriculum_state_path = Path::new(&cfg.paths.curriculum_state_file);
        let current_cfg = match cfg_json {
            Some(json) => json.to_string(),
            None => {
                crate::cfg::execute_cc_json(&cfg.runtime.game, &curriculum_program, "start", None)
            }
        };
        crate::cfg::write_cfg_json(&cfg, &current_cfg);
        let logger = TrainingLogger::new(&cfg.paths.log_dir)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
        let last_logged_episode = crate::logging::csv_max_int(
            &Path::new(&cfg.paths.log_dir).join("episodes.csv"),
            "episode",
            -1,
        )
        .max(crate::logging::jsonl_max_int(
            &Path::new(&cfg.paths.log_dir).join("episodes.jsonl"),
            "episode",
            -1,
        ));
        let update_step = start_update_step;

        let mut episode_count = start_episode;
        if last_logged_episode >= 0 {
            episode_count = episode_count.max(last_logged_episode as usize);
        }
        let episode = Arc::new(Mutex::new(EpisodeCoordinator {
            episode: episode_count,
            cfg: current_cfg.clone(),
            logger,
        }));
        /*
        if last_logged_update > start_update_step as i64 {
            tracing::warn!(
                "src::worker.rs FIXME: Update log is ahead of checkpoint: log={}, checkpoint={}; ",
                last_logged_update,
                start_update_step,
            );
        }

        if let Some(state) = saved_state.as_ref() {
            if state.update_step > start_update_step {
                tracing::warn!(
                    "src::worker.rs FIXME: CC is ahead of checkpoint: log={}, checkpoint={};",
                    state.update_step,
                    start_update_step,
                );
            }
        }
        */
        crate::logging::save_curriculum_state(
            curriculum_state_path,
            &current_cfg,
            episode_count,
            update_step,
        )
        .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
        let control = ControlState {
            paused: false,
            shutdown: false,
            policy_version: update_step,
            config_version: 0,
        };
        // Create control channels (for just a hint message or sth)
        let (control_tx, _) = watch::channel(control);
        let (request_tx, request_rx) = mpsc::channel();
        let (chunk_tx, chunk_rx) = mpsc::channel();
        let paused_workers = Arc::new(
            (0..cfg.num_workers())
                .map(|_| AtomicBool::new(false))
                .collect::<Vec<_>>(),
        );
        let worker_errors = Arc::new(Mutex::new(vec![None; cfg.num_workers()]));
        let mut threads = Vec::with_capacity(cfg.num_workers());
        for worker_id in 0..cfg.num_workers() {
            let cfg_worker = cfg.clone();
            let request_tx_worker = request_tx.clone();
            let chunk_tx_worker = chunk_tx.clone();
            let control_rx = control_tx.subscribe(); // if that is modified
            // they could have seen those.
            let paused_workers_worker = Arc::clone(&paused_workers);
            let worker_errors_worker = Arc::clone(&worker_errors);
            let episode_worker = Arc::clone(&episode);
            let env_spec_worker = env_spec.clone();
            let layout_worker = layout;
            let thread = std::thread::Builder::new()
                .name(format!("rrr-worker-{worker_id}"))
                .spawn(move || {
                    let runtime = tokio::runtime::Builder::new_current_thread()
                        .enable_time()
                        .build();
                    let result = match runtime {
                        Ok(runtime) => runtime.block_on(worker_loop(
                            worker_id,
                            cfg_worker,
                            env_spec_worker,
                            layout_worker,
                            request_tx_worker,
                            chunk_tx_worker,
                            control_rx,
                            paused_workers_worker.clone(),
                            episode_worker,
                        )),
                        Err(err) => Err(err.to_string()),
                    };
                    paused_workers_worker[worker_id].store(false, Ordering::Release);
                    if let Err(err) = result {
                        tracing::error!("w{}: {}", worker_id, err);
                        worker_errors_worker.lock().unwrap()[worker_id] = Some(err);
                    }
                })
                .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
            threads.push(thread);
        }
        tracing::info!("Spawned {} workers", cfg.num_workers());
        Ok(Self {
            cfg,
            request_rx: Mutex::new(request_rx),
            chunk_rx: Mutex::new(chunk_rx),
            pending: Mutex::new(VecDeque::new()),
            control_tx,
            paused_workers,
            worker_errors,
            threads,
            episode,
            curriculum_program,
            env_spec,
            layout,
        })
    }
    /// this function goes from workers, to here and then to Python (the C++ framework).
    pub fn inference_batch<'py>(&self, py: Python<'py>) -> PyResult<Option<Bound<'py, PyDict>>> {
        self.check_worker_error()?; // safety, nothing happened at the night.
        let limit = self.cfg.num_workers();
        let mut envelopes = Vec::with_capacity(limit);
        if limit > 0 {
            let rx = self.request_rx.lock().unwrap();
            match rx.try_recv() {
                Ok(envelope) => envelopes.push(envelope),
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => {
                    return Err(PyRuntimeError::new_err("Inference channel disconnected"));
                }
            }
            while envelopes.len() < limit {
                match rx.try_recv() {
                    Ok(envelope) => envelopes.push(envelope),
                    Err(mpsc::TryRecvError::Empty) => break,
                    Err(mpsc::TryRecvError::Disconnected) => break,
                }
            }
        }
        let current = *self.control_tx.borrow();
        let mut observations = self
            .env_spec
            .observations
            .iter()
            .map(|_| Vec::new())
            .collect::<Vec<Vec<f32>>>();
        let mut state = Vec::with_capacity(envelopes.len() * self.layout.state_size);
        let forced_actions = PyList::empty(py);
        let mut pending = self.pending.lock().unwrap();
        for envelope in envelopes {
            // ... you are not accepting that. or you are wasted. okay... I don't care.
            if envelope.reply.is_closed()
                || envelope.request.policy_version != current.policy_version
                || envelope.request.config_version != current.config_version
                || current.paused
            {
                continue;
            }
            let request = envelope.request;
            for (batch, part) in observations.iter_mut().zip(request.observations.iter()) {
                batch.extend_from_slice(part);
            }
            state.extend_from_slice(&request.state);
            match request.forced_action {
                Some(action) => forced_actions.append(action)?,
                None => forced_actions.append(py.None())?,
            }
            // This is the part I feels good. Normally I will use a single thing and
            // arch for it (burn-rs). This just saved us the fork bomb from pytorch.
            pending.push_back(envelope.reply);
        }
        if forced_actions.len() == 0 {
            return Ok(None);
        }
        let py_observations = PyList::empty(py);
        for part in &observations {
            py_observations.append(PyByteArray::new(py, f32_bytes(part)))?;
        }
        let dict = PyDict::new(py);
        dict.set_item("observations", py_observations)?;
        dict.set_item("state", PyByteArray::new(py, f32_bytes(&state)))?;
        dict.set_item("forced_actions", forced_actions)?;
        Ok(Some(dict))
    }
    /// We go to the opposite position this time.
    pub fn submit_inference(
        &self,
        actions: Vec<usize>,
        aux: Vec<Vec<f32>>,
        state: Vec<Vec<f32>>,
    ) -> PyResult<()> {
        let len = actions.len();
        // safety checks for invalid things.
        if aux.len() != len || state.len() != len {
            return Err(PyRuntimeError::new_err("Invalid inference batch size"));
        }
        if aux.iter().any(|row| row.len() != self.layout.aux_size)
            || state.iter().any(|row| row.len() != self.layout.state_size)
        {
            return Err(PyRuntimeError::new_err(format!(
                "Invalid inference response size: expected aux={}, state={}",
                self.layout.aux_size, self.layout.state_size
            )));
        }
        let mut pending = self.pending.lock().unwrap();
        if pending.len() != len {
            return Err(PyRuntimeError::new_err(
                "Inference response count does not match batch",
            ));
        }
        // give them back to workers.
        for ((action, aux), state) in actions.into_iter().zip(aux).zip(state) {
            if let Some(reply) = pending.pop_front() {
                let _ = reply.send(InferenceResponse { action, aux, state });
            }
        }
        Ok(())
    }

    // Poll chunk_queue for chunks
    pub fn next_rollout<'py>(&self, py: Python<'py>) -> PyResult<Option<Bound<'py, PyDict>>> {
        self.check_worker_error()?;
        loop {
            let chunk = match self.chunk_rx.lock().unwrap().try_recv() {
                Ok(chunk) => chunk,
                Err(mpsc::TryRecvError::Empty) => return Ok(None),
                Err(mpsc::TryRecvError::Disconnected) => {
                    return Err(PyRuntimeError::new_err("Rollout channel disconnected"));
                }
            };
            let current = *self.control_tx.borrow();

            // Reject chunks from old CC config.
            // They may have been produced while I was doing PPO update.
            if chunk.config_version != current.config_version {
                tracing::warn!(
                    "host: Rejected old-config chunk from W{} (chunk cfg v{} vs current cfg v{})",
                    chunk.worker_id,
                    chunk.config_version,
                    current.config_version
                );
                continue;
            }

            // Reject chunks that policy too old.
            let staleness = current.policy_version.saturating_sub(chunk.policy_version);
            // The on policy rejects the chunk anyway so we won't get too many messages in tty.
            if self.cfg.worker.off_policy && staleness > self.cfg.worker.max_policy_staleness {
                tracing::info!(
                    "host: Rejected stale chunk from W{} (policy v{} vs current v{})",
                    chunk.worker_id,
                    chunk.policy_version,
                    current.policy_version
                );
                continue;
            }
            if !self.cfg.worker.off_policy && staleness != 0 {
                continue;
            }
            // and those goes to python things. Why python use PyObject????
            let observations = PyList::empty(py);
            for part in &chunk.observations {
                observations.append(PyByteArray::new(py, f32_bytes(part)))?;
            }
            let dict = PyDict::new(py);
            dict.set_item("observations", observations)?;
            dict.set_item("actions", PyByteArray::new(py, i64_bytes(&chunk.actions)))?;
            dict.set_item("rewards", PyByteArray::new(py, f32_bytes(&chunk.rewards)))?;
            dict.set_item("dones", PyByteArray::new(py, f32_bytes(&chunk.dones)))?;
            dict.set_item("aux", PyByteArray::new(py, f32_bytes(&chunk.aux)))?;
            dict.set_item("state", PyByteArray::new(py, f32_bytes(&chunk.state)))?;
            let tail_observations = PyList::empty(py);
            for part in &chunk.tail_observations {
                tail_observations.append(PyByteArray::new(py, f32_bytes(part)))?;
            }
            dict.set_item("tail_observations", tail_observations)?;
            dict.set_item("tail_aux", PyByteArray::new(py, f32_bytes(&chunk.tail_aux)))?;
            dict.set_item("successes", chunk.successes)?;
            dict.set_item("gameovers", chunk.gameovers)?;
            dict.set_item("n_steps", chunk.len)?;
            return Ok(Some(dict));
        }
    }

    pub fn begin_update(&self) -> PyResult<()> {
        if !self.cfg.worker.off_policy {
            self.pause_collection()?;
        }
        Ok(())
    }

    pub fn prepare_policy_update(&self) -> PyResult<()> {
        self.pause_collection()
    }

    pub fn log_update(&self, record_json: &str) -> PyResult<()> {
        self.episode
            .lock()
            .unwrap()
            .logger
            .log_update_json(record_json)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    pub fn finish_update(
        &self,
        update_step: usize,
        rollout_successes: usize,
        rollout_gameovers: usize,
    ) -> PyResult<()> {
        // CC first. Least instruction.
        let event = if rollout_successes > 0 {
            Some("success")
        } else if rollout_gameovers > 0 {
            Some("fail")
        } else {
            None
        };
        let mut config_changed = false;
        if let Some(event) = event {
            let old_cfg = self.episode.lock().unwrap().cfg.clone();
            let next_cfg = crate::cfg::execute_cc_json(
                &self.cfg.runtime.game,
                &self.curriculum_program,
                event,
                Some(&old_cfg),
            );
            if next_cfg != old_cfg {
                crate::cfg::write_cfg_json(&self.cfg, &next_cfg);
                self.episode.lock().unwrap().cfg = next_cfg.clone();
                config_changed = true;
                tracing::info!(
                    "host: Curriculum {}: {} -> {}",
                    event,
                    crate::cfg::debug_cfg_json(&self.cfg.runtime.game, &old_cfg),
                    crate::cfg::debug_cfg_json(&self.cfg.runtime.game, &next_cfg)
                );
            }
        }
        self.control_tx.send_modify(|state| {
            state.policy_version = update_step;
            if config_changed {
                state.config_version += 1;
            }
        });
        let (episode, cfg) = {
            let state = self.episode.lock().unwrap();
            (state.episode, state.cfg.clone())
        };
        crate::logging::save_curriculum_state(
            Path::new(&self.cfg.paths.curriculum_state_file),
            &cfg,
            episode,
            update_step,
        )
        .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
        self.resume_collection();
        Ok(())
    }

    pub fn episode(&self) -> usize {
        self.episode.lock().unwrap().episode
    }

    pub fn update_step(&self) -> usize {
        self.control_tx.borrow().policy_version
    }

    pub fn cfg_json(&self) -> String {
        self.episode.lock().unwrap().cfg.clone()
    }

    pub fn cfg_debug(&self) -> String {
        crate::cfg::debug_cfg_json(&self.cfg.runtime.game, &self.episode.lock().unwrap().cfg)
    }

    pub fn shutdown(&self) {
        self.control_tx.send_modify(|state| {
            state.paused = false;
            state.shutdown = true;
        });
    }
}

impl Collector {
    /// helper used by another function. Theoritically possible, practically impossible.
    fn check_worker_error(&self) -> PyResult<()> {
        let errors = self.worker_errors.lock().unwrap();
        if let Some((worker_id, err)) = errors
            .iter()
            .enumerate()
            .find_map(|(worker_id, err)| err.as_ref().map(|err| (worker_id, err)))
        {
            return Err(PyRuntimeError::new_err(format!(
                "Worker {worker_id} exited: {err}"
            )));
        }
        drop(errors);
        if let Some((worker_id, _)) = self
            .threads
            .iter()
            .enumerate()
            .find(|(_, thread)| thread.is_finished())
        {
            return Err(PyRuntimeError::new_err(format!(
                "Worker {worker_id} exited"
            )));
        }
        Ok(())
    }

    fn pause_collection(&self) -> PyResult<()> {
        self.check_worker_error()?;
        self.control_tx.send_modify(|state| state.paused = true);
        while !self
            .paused_workers
            .iter()
            .all(|paused| paused.load(Ordering::Acquire))
        {
            self.check_worker_error()?;
            std::thread::sleep(Duration::from_millis(1));
        }
        Ok(())
    }

    fn resume_collection(&self) {
        self.control_tx.send_modify(|state| state.paused = false);
    }
}

impl Drop for Collector {
    fn drop(&mut self) {
        self.control_tx.send_modify(|state| {
            state.paused = false;
            state.shutdown = true;
        });
        self.pending.lock().unwrap().clear();
        for thread in self.threads.drain(..) {
            let _ = thread.join();
        }
    }
}

// still remembered the first thing of my javascript is a async fn.
/// this is worker loop, a shit function by me. It still contains
/// game-specific things and I will delete them, but not in v0.2.0.
async fn worker_loop(
    worker_id: usize,
    cfg: RuntimeConfig,
    spec: EnvSpec,
    layout: AlgorithmLayout,
    request_tx: mpsc::Sender<InferenceEnvelope>,
    chunk_tx: mpsc::Sender<RolloutChunk>,
    mut control: watch::Receiver<ControlState>,
    paused_workers: Arc<Vec<AtomicBool>>,
    episode: Arc<Mutex<EpisodeCoordinator>>,
) -> Result<(), String> {
    tracing::info!(
        "w{}: Starting worker at policy version {}",
        worker_id,
        control.borrow().policy_version
    );
    let mut env = crate::games::spawn(&cfg)?;
    let mut state = env.initial_state()?;
    if state.is_none() {
        return Err("Worker could not get initial state. Is it killed?".to_string());
    }
    let mut policy_state = vec![0.0f32; layout.state_size];
    let mut buffer = RolloutBuffer::new(&cfg, &spec, layout);
    // Is total reward, not seperate obj R.
    let mut ep_rewards = vec![0.0f64; spec.rewards];
    let mut ep_len = 0usize;
    let mut local_policy_ver = control.borrow().policy_version;
    let mut local_config_ver = control.borrow().config_version;

    'worker: loop {
        let current = *control.borrow_and_update();
        if current.shutdown {
            break;
        }
        if current.paused {
            paused_workers[worker_id].store(true, Ordering::Release);
            if control.changed().await.is_err() {
                break;
            }
            continue;
        }
        paused_workers[worker_id].store(false, Ordering::Release);
        if current.config_version != local_config_ver {
            tokio::time::sleep(Duration::from_millis(cfg.worker.restart_sleep_ms)).await;
            env = crate::games::spawn(&cfg)?;
            state = env.initial_state()?;
            policy_state.fill(0.0);
            buffer.clear();
            ep_rewards.fill(0.0);
            ep_len = 0;
            local_config_ver = current.config_version;
            local_policy_ver = current.policy_version;
            tracing::info!("w{}: Config v{}, restarting.", worker_id, local_config_ver);
            continue;
        }
        if current.policy_version != local_policy_ver {
            policy_state.fill(0.0);
            buffer.clear();
            local_policy_ver = current.policy_version;
            tracing::info!("w{}: Policy updated to v{}", worker_id, local_policy_ver);
            continue;
        }
        if state.is_none() {
            state = env.initial_state()?;
            if state.is_none() {
                continue;
            }
        }
        let Some(current_state) = state.as_ref() else {
            continue;
        };
        let state_before = policy_state.clone();
        // I mean this
        let forced_action = if cfg.runtime.rl_by_human {
            // Read the actual action taken by me.
            env.human_action()?
        } else {
            None
        };
        let frame_started = tokio::time::Instant::now();
        let (reply_tx, mut reply_rx) = oneshot::channel();
        request_tx
            .send(InferenceEnvelope {
                request: InferenceRequest {
                    policy_version: local_policy_ver,
                    config_version: local_config_ver,
                    observations: current_state.observations.clone(),
                    state: state_before.clone(),
                    forced_action,
                },
                reply: reply_tx,
            })
            .map_err(|_| "Inference request channel disconnected".to_string())?;
        let response = loop {
            tokio::select! {
                response = &mut reply_rx => {
                    match response {
                        Ok(response) => break Some(response),
                        Err(_) => break None,
                    }
                }
                changed = control.changed() => {
                    if changed.is_err() {
                        break 'worker;
                    }
                    let changed = *control.borrow_and_update();
                    if changed.shutdown {
                        break 'worker;
                    }
                    if changed.paused
                        || changed.policy_version != local_policy_ver
                        || changed.config_version != local_config_ver
                    {
                        break None;
                    }
                }
            }
        };
        let Some(response) = response else {
            continue;
        };
        if buffer.len >= cfg.worker.chunk_size {
            let chunk = buffer.take(
                worker_id,
                local_policy_ver,
                local_config_ver,
                current_state.observations.clone(),
                response.aux.clone(),
            );
            chunk_tx
                .send(chunk)
                .map_err(|_| "Chunk channel disconnected".to_string())?;
        }
        policy_state = response.state;

        // move such a critical loop into rust. if burn-rs, all solved... but I still have xpu...
        // Literally zero money to buy a cuda, also idk Computer or Laptop... due to the need of Matrix
        // Multiplication Computing those years. DDR4 or lower will not be acceptable I tho.
        let next_state = env.step(response.action, frame_started.elapsed())?;
        let Some(mut next_state) = next_state else {
            policy_state.fill(0.0);
            buffer.clear();
            state = None;
            continue;
        };

        // Calc R and all transitions (into chunk).
        let next_done_flag = next_state.end;
        for (reward, scale) in next_state.rewards.iter_mut().zip(cfg.reward.scales.iter()) {
            *reward *= *scale;
        }
        for (dst, src) in buffer
            .observations
            .iter_mut()
            .zip(current_state.observations.iter())
        {
            dst.extend_from_slice(src);
        }
        buffer.actions.push(response.action as i64);
        buffer.rewards.extend_from_slice(&next_state.rewards);
        // this, the next_done_flag
        buffer
            .dones
            .push(if next_done_flag != 0 { 1.0 } else { 0.0 });
        buffer.aux.extend_from_slice(&response.aux);
        buffer.state.extend_from_slice(&state_before);
        buffer.len += 1;
        buffer.last_done = next_done_flag != 0;

        for (total, reward) in ep_rewards.iter_mut().zip(next_state.rewards.iter()) {
            *total += f64::from(*reward);
        }
        ep_len += 1;

        if next_done_flag != 0 {
            if next_done_flag == 2 {
                buffer.successes += 1;
            } else {
                buffer.gameovers += 1;
            }
            tracing::info!(
                "w{}: {} R=[{:.1},{:.1},{:.1}] L={}",
                worker_id,
                if next_done_flag == 2 {
                    "Success"
                } else {
                    "GameOver"
                },
                // and this.
                ep_rewards.first().copied().unwrap_or_default(),
                ep_rewards.get(1).copied().unwrap_or_default(),
                ep_rewards.get(2).copied().unwrap_or_default(),
                ep_len
            );
            let rewards = ep_rewards
                .iter()
                .map(|value| *value as f32)
                .collect::<Vec<_>>();
            episode
                .lock()
                .unwrap()
                .record(&rewards, ep_len, next_done_flag);
            ep_rewards.fill(0.0);
            ep_len = 0;
        }

        let tail_observations = next_state.observations.clone();
        state = Some(next_state);
        if buffer.len >= cfg.worker.chunk_size && buffer.last_done {
            let chunk = buffer.take(
                worker_id,
                local_policy_ver,
                local_config_ver,
                tail_observations,
                vec![0.0; layout.aux_size],
            );
            chunk_tx
                .send(chunk)
                .map_err(|_| "Chunk channel disconnected".to_string())?;
        }

        if next_done_flag != 0 {
            policy_state.fill(0.0);
            state = env.reset()?;
            if control.borrow().shutdown {
                break;
            }
        }
    }
    paused_workers[worker_id].store(false, Ordering::Release);
    Ok(())
}

// bytemuck??? might use it for now. +1 is always a step of risk of supply chains.
fn f32_bytes(values: &[f32]) -> &[u8] {
    unsafe {
        std::slice::from_raw_parts(values.as_ptr().cast::<u8>(), std::mem::size_of_val(values))
    }
}

fn i64_bytes(values: &[i64]) -> &[u8] {
    unsafe {
        std::slice::from_raw_parts(values.as_ptr().cast::<u8>(), std::mem::size_of_val(values))
    }
}
