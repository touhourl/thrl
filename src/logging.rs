//! Training data logging to CSV files. With training resume
//! and log overwrite.
//!
//! The last file was very messy and does not meet
//! production rules. Spagetti rust was it.
//!
//! [paper]
//! [copy]
//! [rewrite:80]
//! [basic]
//!
//! not used anymore ... i wanna delete it but we might have furture rewrite,
//! if the burn.rs has way better xpu support than vulkan / wgpu ... Imagine

/*
    Logging of RL-rs.
    Logging of thrl.
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

// Huh, typical markdown.

use crate::error::LogError;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
// begin new code for rrr
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EpisodeState {
    Running,
    Success,
    GameOver,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurriculumState {
    pub cfg: serde_json::Value,
    pub episode: usize,
    pub update_step: usize,
}

pub struct TrainingLogger {
    log_dir: PathBuf,
}
// no changes from RL-rs, or very little adapt of the structs.
impl TrainingLogger {
    pub fn new(log_dir: &str) -> Result<Self, LogError> {
        let dir = PathBuf::from(log_dir);
        fs::create_dir_all(&dir)?;
        Ok(Self { log_dir: dir })
    }

    /// Log an episode end event. Overwrites if episode already exists.
    /// Also removes any episodes >= current episode to handle checkpoint resets.
    pub fn log_episode_json(
        &self,
        episode: usize,
        mo_r: &[f32],
        length: usize,
        final_state: EpisodeState,
        cfg_json: &str,
    ) -> Result<(), LogError> {
        let path = self.log_dir.join("episodes.jsonl");
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        let record = serde_json::json!({
            "episode": episode,
            "rewards": mo_r,
            "length": length,
            "final_state": format!("{:?}", final_state),
            "cfg": serde_json::from_str::<serde_json::Value>(cfg_json).map_err(|e| LogError::Parse(e.to_string()))?,
        });
        use std::io::Write;
        writeln!(file, "{}", record)?;
        Ok(())
    }

    /// Log a PPO update. Appends to updates CSV (no rewrite).
    pub fn log_update_json(&self, record_json: &str) -> Result<(), LogError> {
        let path = self.log_dir.join("updates.jsonl");
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        use std::io::Write;
        writeln!(file, "{}", record_json)?;
        Ok(())
    }
}

/// Load episodes from CSV into memory.
/// No, I mean `mov rax, [0x4B534F]`
pub fn csv_max_int(path: &Path, column: &str, default: i64) -> i64 {
    if !path.exists() {
        return default;
    }
    let file = match fs::File::open(path) {
        Ok(file) => file,
        Err(_) => return default,
    };
    let mut reader = csv::Reader::from_reader(file);
    let headers = match reader.headers() {
        Ok(headers) => headers.clone(),
        Err(_) => return default,
    };
    let Some(index) = headers.iter().position(|value| value == column) else {
        return default;
    };
    let mut best = default;
    for row in reader.records().flatten() {
        if let Some(value) = row.get(index).and_then(|value| value.parse::<i64>().ok()) {
            best = best.max(value);
        }
    }
    best
}
// this is also new. use jsonl crate?? but bro look at edition = "2018" and nope.
pub fn jsonl_max_int(path: &Path, field: &str, default: i64) -> i64 {
    let Ok(data) = fs::read_to_string(path) else {
        return default;
    };
    data.lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .filter_map(|value| value.get(field).and_then(serde_json::Value::as_i64))
        .fold(default, i64::max)
}

pub fn save_curriculum_state(
    path: &Path,
    cfg_json: &str,
    episode: usize,
    update_step: usize,
) -> Result<(), LogError> {
    let state = CurriculumState {
        cfg: serde_json::from_str(cfg_json).map_err(|e| LogError::Parse(e.to_string()))?,
        episode,
        update_step,
    };
    let tmp = PathBuf::from(format!("{}.tmp", path.display()));
    let data = serde_json::to_vec_pretty(&state).map_err(|e| LogError::Parse(e.to_string()))?;
    fs::write(&tmp, data)?;
    fs::rename(tmp, path)?;
    Ok(())
}

pub fn load_curriculum_state(path: &Path) -> Result<Option<CurriculumState>, LogError> {
    if !path.exists() {
        return Ok(None);
    }
    let data = fs::read(path)?;
    let state = serde_json::from_slice(&data).map_err(|e| LogError::Parse(e.to_string()))?;
    Ok(Some(state))
}

pub fn init_tracing() {
    use tracing_subscriber::EnvFilter;

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();
}
