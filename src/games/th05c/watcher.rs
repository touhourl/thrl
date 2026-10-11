use super::key::{DiscreteAction, MoveAction, key_det_player_pos, shiftkey_player_pos};
use super::readers::read_state;
use super::{DynAddressFinder, GameState};
use crate::games::Environment;
use crate::observation::{Frame, Schema};
use crate::param::RuntimeConfig;
use std::io::ErrorKind;
use std::time::{Duration, Instant};

/*
    TH05 memory watcher of thrl.
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
pub struct TH05MemoryWatcher {
    pub finder: DynAddressFinder,
    pub current_action: Option<usize>,
    pub keyboard_device: Option<evdev::Device>,
}
/// We do here sh*t code: one over another...
impl TH05MemoryWatcher {
    pub fn new(pid: i32) -> Result<Self, Box<dyn std::error::Error>> {
        let finder = DynAddressFinder::new(pid)?;
        let mut keyboard_device = None;
        if let Ok(dir) = std::fs::read_dir("/dev/input") {
            for entry in dir.flatten() {
                let path = entry.path();
                if path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|s| s.starts_with("event"))
                {
                    // check
                    if let Ok(device) = evdev::Device::open(&path) {
                        // Check
                        if let Some(keys) = device.supported_keys()
                            && keys.contains(evdev::KeyCode::KEY_UP)
                            && keys.contains(evdev::KeyCode::KEY_X)
                        {
                            keyboard_device = Some(device);
                            break;
                        }
                    }
                }
            }
        }

        Ok(Self {
            finder,
            current_action: None,
            keyboard_device,
        })
    }

    pub fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.finder.all_structures()?;
        Ok(())
    }

    pub fn try_read_state(&mut self) -> Option<GameState> {
        read_state(&mut self.finder)
    }

    pub fn apply_action(&mut self, action: usize) -> Result<(), Box<dyn std::error::Error>> {
        let discrete_action = DiscreteAction::from_index(action).ok_or("Invalid action index")?;
        let player_pos = self
            .finder
            .addresses()
            .player_pos
            .ok_or("Player pos not found")?;
        let key_det_addr = key_det_player_pos(player_pos);
        let shiftkey_addr = shiftkey_player_pos(player_pos);

        discrete_action.apply(self.finder.memory(), key_det_addr, shiftkey_addr)?; // call
        self.current_action = Some(action);
        Ok(())
    }

    pub fn release_action(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.current_action.is_some() {
            let player_pos = self
                .finder
                .addresses()
                .player_pos
                .ok_or("Player pos not found")?;
            let key_det_addr = key_det_player_pos(player_pos);
            let shiftkey_addr = shiftkey_player_pos(player_pos);

            // Write zero to release all keys
            self.finder.memory().write_u16_le(key_det_addr, 0)?;
            self.finder.memory().write_u8(shiftkey_addr, 0)?;
            self.current_action = None;
        }
        Ok(())
    }
}

#[derive(serde::Deserialize)]
struct TH05WorkerSettings {
    frame_interval_ms: u64,
    action_repeat_sleep_ms: u64,
    episode_cooldown_ms: u64,
    initial_state_attempts: usize,
    initial_state_sleep_ms: u64,
    reconnect_attempts: usize,
    reconnect_sleep_ms: u64,
}

pub struct TH05CSession {
    pid: i32,
    inner: TH05MemoryWatcher,
    child: Option<std::process::Child>,
    frame_interval: Duration,
    action_repeat_sleep: Duration,
    episode_cooldown: Duration,
    initial_state_attempts: usize,
    initial_state_sleep: Duration,
    reconnect_attempts: usize,
    reconnect_sleep: Duration,
    schema: Schema,
}

impl Drop for TH05CSession {
    fn drop(&mut self) {
        if let Some(ref mut child) = self.child {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

impl TH05CSession {
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

    pub fn release_action(&mut self) -> Result<(), String> {
        self.inner.release_action().map_err(|e| e.to_string())
    }

    pub fn apply_action(&mut self, action: usize) -> Result<(), String> {
        self.inner.apply_action(action).map_err(|e| e.to_string())
    }

    /// Read the current game state and return MO rewards.
    /// if you want more documents please read docs dir.
    pub fn read_state(&mut self) -> Option<GameState> {
        self.inner.try_read_state()
    }

    fn observation_frame(&mut self) -> Result<Option<Frame>, String> {
        let Some(state) = self.read_state() else {
            return Ok(None);
        };
        super::observation::frame(self.schema, state).map(Some)
    }

    /// Read the keys pressed by me.
    pub fn read_human_action(&mut self) -> Result<usize, String> {
        let Some(ref mut device) = self.inner.keyboard_device else {
            return Err(
                "No keyboard device found. Is your kernel outdated? How you started Linux?"
                    .to_string(),
            );
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
                return Err(format!("Failed to fetch keyboard events: {err}"));
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
            MoveAction::LeftUp
        } else if move_left && move_down {
            MoveAction::LeftDown
        } else if move_right && move_up {
            MoveAction::RightUp
        } else if move_right && move_down {
            MoveAction::RightDown
        } else if move_left {
            MoveAction::Left
        } else if move_right {
            MoveAction::Right
        } else if move_up {
            MoveAction::Up
        } else if move_down {
            MoveAction::Down
        } else {
            MoveAction::Idle
        };

        let action = DiscreteAction::Move { movement, shift };
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

    fn cfg_keydev(inner: &mut TH05MemoryWatcher) -> Result<(), String> {
        let Some(device) = inner.keyboard_device.as_mut() else {
            return Ok(());
        };

        device
            .set_nonblocking(true)
            .map_err(|e| format!("Failed to set keyboard device nonblocking: {e}"))?;

        // there are no pending keyboard events
        match device.fetch_events() {
            Ok(events) => {
                for _event in events {
                    // Again thanks
                }
            }
            Err(err) if err.kind() == ErrorKind::WouldBlock => {}
            Err(err) => {
                return Err(format!("Failed to fetch initial keyboard events: {err}"));
            }
        }

        Ok(())
    }

    pub fn spawn(cfg: &RuntimeConfig) -> Result<Self, String> {
        use std::process::Command;

        let worker: TH05WorkerSettings = cfg.raw["worker"].clone().try_into().unwrap();
        let schema = Schema::try_from(cfg.runtime.schema)?;
        let export_dir = cfg.paths.export_dir.clone();

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
            .map_err(|e| format!("Failed to spawn dosbox-x: {}", e))?;

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
                            frame_interval: Duration::from_millis(worker.frame_interval_ms),
                            action_repeat_sleep: Duration::from_millis(
                                worker.action_repeat_sleep_ms,
                            ),
                            episode_cooldown: Duration::from_millis(worker.episode_cooldown_ms),
                            initial_state_attempts: worker.initial_state_attempts,
                            initial_state_sleep: Duration::from_millis(
                                worker.initial_state_sleep_ms,
                            ),
                            reconnect_attempts: worker.reconnect_attempts,
                            reconnect_sleep: Duration::from_millis(worker.reconnect_sleep_ms),
                            schema,
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
        Err(format!(
            "DOSBox-X (PID {}) started but game structures not found after {}: {}. \
                Are child running? Have you built dosbox-x from source?",
            pid, max_retries, last_err
        ))
    }
}

impl Environment for TH05CSession {
    fn initial_state(&mut self) -> Result<Option<Frame>, String> {
        self.resume_game();
        for _ in 0..self.initial_state_attempts {
            if let Some(state) = self.observation_frame()? {
                self.pause_game();
                return Ok(Some(state));
            }
            std::thread::sleep(self.initial_state_sleep);
        }
        Ok(None)
    }

    fn step(&mut self, action: usize, frame_elapsed: Duration) -> Result<Option<Frame>, String> {
        let remaining_budget = self.frame_interval.saturating_sub(frame_elapsed);
        let started = Instant::now();
        self.apply_action(action)?;
        self.resume_game();
        while started.elapsed() < remaining_budget {
            self.apply_action(action)?;
            let remaining = remaining_budget.saturating_sub(started.elapsed());
            std::thread::sleep(remaining.min(self.action_repeat_sleep));
        }
        self.pause_game();
        self.observation_frame()
    }

    fn reset(&mut self) -> Result<Option<Frame>, String> {
        self.release_action()?;
        self.resume_game();
        std::thread::sleep(self.episode_cooldown);
        for _ in 0..self.reconnect_attempts {
            if let Some(state) = self.observation_frame()? {
                self.pause_game();
                return Ok(Some(state));
            }
            std::thread::sleep(self.reconnect_sleep);
        }
        Ok(None)
    }

    fn human_action(&mut self) -> Result<Option<usize>, String> {
        self.read_human_action().map(Some)
    }
}
