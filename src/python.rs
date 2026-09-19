use crate::games::th05_c::GameState;
use crate::games::th05_c::watcher::TH05MemoryWatcher;
use crate::observation::schema1::ObservationBuilder;
use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3::types::PyBytes;
use serde::{Deserialize, Serialize};
use std::io::ErrorKind;

/*
    Rust-Python Bindings of rrr.
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

/// It is a snapshort which has not been passed to the map generator, so it is quite
/// small and is good for sitting in ram. When needed, pass it back to map.
#[pyclass(module = "rrr")]
#[derive(Clone, Serialize, Deserialize)]
pub struct RawFrame {
    pub(crate) state: GameState,
}
#[allow(clippy::new_without_default)]
// You clippy self touch the code here and got beated...
#[pymethods]
impl RawFrame {
    pub fn __getstate__<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyBytes>> {
        let data = serde_json::to_vec(&self.state)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        Ok(PyBytes::new(py, &data))
    }

    pub fn __setstate__(&mut self, state: &Bound<'_, PyBytes>) -> PyResult<()> {
        let data = state.as_bytes();
        self.state = serde_json::from_slice(data)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        Ok(())
    }

    #[new]
    pub fn new() -> Self {
        Self {
            state: GameState {
                // [copy]

                // reader.rs
                resident: crate::games::th05_c::types::ResidentState {
                    rem_lives: 0,
                    credit_lives: 0,
                    rem_bombs: 0,
                    credit_bombs: 0,
                    stage: 0,
                    rank: 0,
                    playchar: 0,
                    stage_ascii: 0,
                    rand: 0,
                    bgm_mode: 0,
                    se_mode: 0,
                    shottype: 0,
                    debug_mode: 0,
                    end_type_ascii: 0,
                    end_sequence: 0,
                    score: 0,
                    graze: 0,
                    miss_count: 0,
                    bombs_used: 0,
                    items_spawned: 0,
                    items_collected: 0,
                    point_items_collected: 0,
                    max_valued_point_items_collected: 0,
                    enemies_killed: 0,
                    enemies_gone: 0,
                    frames: 0,
                    slow_frames: 0,
                    std_frames: 0,
                    demo_stage: 0,
                    demo_num: 0,
                    zunsoft_shown: 0,
                    turbo_mode: 0,
                    game_end_flag: 0,
                },
                player: crate::games::th05_c::types::PlayerState {
                    pos: crate::games::th05_c::types::PlayfieldMotion {
                        cur_x: 0,
                        cur_y: 0,
                        prev_x: 0,
                        prev_y: 0,
                        vel_x: 0,
                        vel_y: 0,
                    },
                    power: 0,
                    invincibility_time: 0,
                    invincible_via_bomb: false,
                    miss_frame: 0,
                },
                bullets: vec![],
                enemies: vec![],
                items: vec![],
                boss: None,
                boss_2: None,
                midboss: None,
                lasers: vec![],
                cheeto_trails: vec![],
                custom_entities: vec![],
                firewaves: vec![],
                stage_collection: Default::default(),
                rem_bombs_internal: 0,
            },
        }
    }

    pub fn __repr__(&self) -> String {
        format!(
            "RawFrame(stage={}, frame={})",
            self.state.resident.stage, self.state.resident.frames
        )
    }
}

#[pyclass]
pub struct MemoryWatcher {
    pid: i32,
    inner: TH05MemoryWatcher,
    child: Option<std::process::Child>,
}

impl Drop for MemoryWatcher {
    fn drop(&mut self) {
        if let Some(ref mut child) = self.child {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

#[pymethods]
impl MemoryWatcher {
    /// If spawn_dosbox is True, a dosboxx process is spawned as a
    /// child and attached to main directly. So I would not require perfect timing
    /// to launch the agent.
    #[new]
    #[pyo3(signature = (spawn_dosbox=false))]
    pub fn new(spawn_dosbox: bool) -> PyResult<Self> {
        if spawn_dosbox {
            Self::new_spawn()
        } else {
            Self::new_attach()
        }
    }

    pub fn pause_game(&self) {
        let _ = nix::sys::signal::kill(
            nix::unistd::Pid::from_raw(self.pid),
            nix::sys::signal::Signal::SIGSTOP,
        );
    }

    pub fn resume_game(&self) {
        let _ = nix::sys::signal::kill(
            nix::unistd::Pid::from_raw(self.pid),
            nix::sys::signal::Signal::SIGCONT,
        );
    }

    pub fn release_action(&mut self) -> PyResult<()> {
        self.inner
            .release_action()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
    }

    pub fn apply_action(&mut self, action: usize) -> PyResult<()> {
        self.inner
            .apply_action(action)
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
    }

    /// Read the current game state and return MO rewards.
    /// if you want more documents please read docs dir.
    pub fn read_state(&mut self) -> PyResult<Option<(Vec<f32>, Vec<f32>, u8, Vec<f32>, RawFrame)>> {
        let prev_state = self.inner.last_state.clone();

        let state = match self.inner.try_read_state() {
            Some(s) => s,
            None => return Ok(None),
        };

        let (features, maps) = self.inner.observation_comp(&state);
        let game_end_flag = state.resident.game_end_flag;

        let rewards = if let Some(ref prev) = prev_state {
            let mut r = crate::observation::schema1::reward::calculate_reward_m(Some(prev), &state);
            // Apply episode-end good things
            if game_end_flag == 1 {
                r[0] -= 100.0;
            } else if game_end_flag == 2 {
                r[0] += 100.0;
            }
            r
        } else {
            vec![0.0, 0.0, 0.0]
        };

        let raw_frame = RawFrame { state };
        Ok(Some((features, maps, game_end_flag, rewards, raw_frame)))
    }

    pub fn clear_state(&mut self) {
        self.inner.last_state = None;
    }

    /// Read the keys pressed by me.
    pub fn read_human_action(&mut self) -> PyResult<usize> {
        let Some(ref mut device) = self.inner.keyboard_device else {
            return Err(PyRuntimeError::new_err(
                "No keyboard device found. Is your kernel outdated? How you started Linux?",
            ));
        };
        // non blocking for the less latency cuz I won't hold bomb key all the time like it.
        match device.fetch_events() {
            Ok(events) => {
                for _event in events {
                    // thanks for the optimization!
                }
            }
            Err(err) if err.kind() == ErrorKind::WouldBlock => {
                // No new keyboard events this frame.
            }
            Err(err) => {
                return Err(PyRuntimeError::new_err(format!(
                    "Failed to fetch keyboard events: {err}"
                )));
            }
        }

        let state = device.cached_state();

        let is_pressed = |key: evdev::KeyCode| -> bool {
            state.key_vals().is_some_and(|keys| keys.contains(key))
        };

        let up = is_pressed(evdev::KeyCode::KEY_UP);
        let down = is_pressed(evdev::KeyCode::KEY_DOWN);
        let left = is_pressed(evdev::KeyCode::KEY_LEFT);
        let right = is_pressed(evdev::KeyCode::KEY_RIGHT);
        let bomb = is_pressed(evdev::KeyCode::KEY_X);
        let shift =
            is_pressed(evdev::KeyCode::KEY_LEFTSHIFT) || is_pressed(evdev::KeyCode::KEY_RIGHTSHIFT);

        if bomb {
            return Ok(18); // bomb idx is 18
        }

        let move_left = left && !right;
        let move_right = right && !left;
        let move_up = up && !down;
        let move_down = down && !up;

        let movement = if move_left && move_up {
            crate::games::th05_c::key::MoveAction::LeftUp
        } else if move_left && move_down {
            crate::games::th05_c::key::MoveAction::LeftDown
        } else if move_right && move_up {
            crate::games::th05_c::key::MoveAction::RightUp
        } else if move_right && move_down {
            crate::games::th05_c::key::MoveAction::RightDown
        } else if move_left {
            crate::games::th05_c::key::MoveAction::Left
        } else if move_right {
            crate::games::th05_c::key::MoveAction::Right
        } else if move_up {
            crate::games::th05_c::key::MoveAction::Up
        } else if move_down {
            crate::games::th05_c::key::MoveAction::Down
        } else {
            crate::games::th05_c::key::MoveAction::Idle
        };

        let action = crate::games::th05_c::key::DiscreteAction::Move { movement, shift };
        Ok(action.to_index())
    }

    /// Terminate (or, SIGKILL actually) the managed DOSBox-X child process, if any.
    pub fn terminate(&mut self) {
        if let Some(ref mut child) = self.child {
            let _ = child.kill();
            let _ = child.wait();
        }
        self.child = None;
    }
    pub fn pid(&self) -> i32 {
        self.pid
    }
}

impl MemoryWatcher {
    fn cfg_keydev(inner: &mut TH05MemoryWatcher) -> PyResult<()> {
        let Some(device) = inner.keyboard_device.as_mut() else {
            return Ok(());
        };

        device.set_nonblocking(true).map_err(|e| {
            PyRuntimeError::new_err(format!("Failed to set keyboard device nonblocking: {e}"))
        })?;

        // there are no pending keyboard events
        match device.fetch_events() {
            Ok(events) => {
                for _event in events {
                    // Again thanks
                }
            }
            Err(err) if err.kind() == ErrorKind::WouldBlock => {}
            Err(err) => {
                return Err(PyRuntimeError::new_err(format!(
                    "Failed to fetch initial keyboard events: {err}"
                )));
            }
        }

        Ok(())
    }

    /// Attach to one, very old.
    fn new_attach() -> PyResult<Self> {
        let pid = crate::memory::find_pid()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        let mut inner = TH05MemoryWatcher::new(pid)
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        inner
            .initialize()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

        Ok(Self {
            pid,
            inner,
            child: None,
        })
    }

    fn new_spawn() -> PyResult<Self> {
        use std::process::Command;

        let export_dir = std::env::current_dir()
            .map_err(|e| {
                pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to get cwd: {}", e))
            })?
            .join("export");

        let child = Command::new("dosbox-x")
            .args([
                ".",
                "-conf",
                "./default.conf",
                "-c",
                "mount c .",
                "-c",
                "c:",
                "-c",
                "game",
                "-fastlaunch",
            ])
            .current_dir(&export_dir)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|e| {
                pyo3::exceptions::PyRuntimeError::new_err(format!(
                    "Failed to spawn dosbox-x: {}",
                    e
                ))
            })?;

        let pid = child.id() as i32;
        tracing::info!("Spawned DOSBox-X child process with PID {}", pid);

        // Let it start and wait for bios and game render.
        std::thread::sleep(std::time::Duration::from_millis(500));

        // cuz sometimes it is slow so retry.
        let max_retries = 30;
        let mut last_err = String::new();
        for attempt in 1..=max_retries {
            match TH05MemoryWatcher::new(pid) {
                Ok(mut inner) => match inner.initialize() {
                    Ok(()) => {
                        Self::cfg_keydev(&mut inner)?;

                        tracing::info!(
                            "MemoryWatcher attached to PID {} after {} attempt(s)",
                            pid,
                            attempt
                        );

                        return Ok(Self {
                            pid,
                            inner,
                            child: Some(child),
                        });
                    }
                    Err(e) => {
                        last_err = e.to_string();
                    }
                },
                Err(e) => {
                    last_err = e.to_string();
                }
            }
            if attempt < max_retries {
                std::thread::sleep(std::time::Duration::from_millis(500));
            }
        }

        // Failed after all retries.
        let mut child = child;
        let _ = child.kill();
        let _ = child.wait();
        Err(pyo3::exceptions::PyRuntimeError::new_err(format!(
            "DOSBox-X (PID {}) started but game structures not found after {}: {}. \
                Are child running? Have you built dosbox-x from source?",
            pid, max_retries, last_err
        )))
    }
}

/// Resolve a curriculum event from jason. `program` is the cc from Python (i cannot name it
/// better)
/// btw, specific cfg gen is not a good name for that, really.
#[pyfunction]
#[pyo3(signature = (program, event, current_json=None))]
pub fn cfg_execute(program: &str, event: &str, current_json: Option<&str>) -> String {
    crate::cfg::execute_cc_json(program, event, current_json)
}

/// Write a json coded directly
#[pyfunction]
pub fn cfg_write_json(path: &str, cfg_json: &str) {
    crate::cfg::write_cfg_json(std::path::Path::new(path), cfg_json)
}

#[pyfunction]
pub fn init_logging() {
    crate::logging::init_tracing();
}

/// Reconstruct spatial maps from before the raw frames, direct game raw frame.
/// So why is it not used? Well, in 1ms it has already done. So no need.
/// (Unless you have serious memory lackages)
#[pyfunction]
pub fn rec_maps_bytes<'py>(
    py: Python<'py>,
    frames: Vec<PyRef<'_, RawFrame>>,
) -> PyResult<Bound<'py, PyBytes>> {
    let builder = ObservationBuilder::default();
    let num_frames = frames.len();
    let size = builder.grid_w * builder.grid_h * 24; // 211968 ?
    let mut buffer = vec![0.0f32; num_frames * size];

    for (i, f) in frames.iter().enumerate() {
        let obs = builder.build_observation(&f.state);
        let tensor = obs.to_map_tensor();
        let start = i * size;
        buffer[start..start + size].copy_from_slice(&tensor);
    }

    let byte_slice = unsafe {
        std::slice::from_raw_parts(
            buffer.as_ptr() as *const u8,
            buffer.len() * std::mem::size_of::<f32>(),
        )
    };

    Ok(PyBytes::new(py, byte_slice))
}

#[pymodule]
fn rrr(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<MemoryWatcher>()?;
    m.add_class::<RawFrame>()?;
    m.add_function(wrap_pyfunction!(cfg_execute, m)?)?;
    m.add_function(wrap_pyfunction!(cfg_write_json, m)?)?;
    m.add_function(wrap_pyfunction!(init_logging, m)?)?;
    m.add_function(wrap_pyfunction!(rec_maps_bytes, m)?)?;
    Ok(())
}
