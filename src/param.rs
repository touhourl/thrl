//! Parameter constants. Was `param.rs` before.
//! This file provides compiled configs
//! so we just need to modify it while changing things
//! like game variable, logging, etc
//!

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::OnceLock;

/*
    Parameter constants of RL-rs, rrr, thrl.
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

// If we say python is PyObjects/dics, C is Pointers, C++ is <(::)>s,
// HTML is </>s, JavaScript is [object Object]s, then, rust is just structs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfig {
    #[serde(skip)]
    pub raw: toml::Table,
    pub runtime: RuntimeSettings,
    pub paths: PathSettings,
    #[serde(skip_serializing)]
    pub worker: WorkerSettings,
    pub reward: RewardSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeSettings {
    // === Live adjustment after training started === #
    pub rl_by_human: bool,
    pub game: String,
    pub schema: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathSettings {
    // === PATH === #
    pub log_dir: String,
    pub curriculum_file: String,
    pub curriculum_state_file: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerSettings {
    // === WORKER SETTINGS === #
    pub num_workers: usize,
    pub human_num_workers: usize,
    pub chunk_size: usize,
    pub off_policy: bool,
    pub max_policy_staleness: usize,
    pub restart_sleep_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardSettings {
    // === REWARD === # TODO: do it in rust. This is test and experimental, and I need quick changes. Compiling and wait the `uv` to finish managing packages are such a pain
    pub scales: Vec<f32>,
}

static RUNTIME_CONFIG: OnceLock<RuntimeConfig> = OnceLock::new();

impl RuntimeConfig {
    pub fn load() -> Result<Self, String> {
        let path = std::env::var_os("RRR_CONFIG")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("rrr.toml"));
        let text = std::fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read {}: {e}", path.display()))?;
        let raw: toml::Table = toml::from_str(&text)
            .map_err(|e| format!("Failed to parse {}: {e}", path.display()))?;
        let mut cfg: Self = toml::from_str(&text)
            .map_err(|e| format!("Failed to parse {}: {e}", path.display()))?;
        cfg.raw = raw;
        std::fs::create_dir_all(&cfg.paths.log_dir).map_err(|e| e.to_string())?;
        Ok(cfg)
    }

    pub fn global() -> &'static Self {
        RUNTIME_CONFIG.get_or_init(|| Self::load().unwrap_or_else(|e| panic!("{e}")))
    }

    pub fn num_workers(&self) -> usize {
        if self.runtime.rl_by_human {
            self.worker.human_num_workers
        } else {
            self.worker.num_workers
        }
    }
}

pub struct ObservationConfig;
impl ObservationConfig {
    pub const GRID_W: usize = 96;
    pub const GRID_H: usize = 92;
    pub const SPAN_X_PX: f32 = 384.0;
    pub const SPAN_Y_PX: f32 = 368.0;
}
pub const STAGE_WEIGHT: f32 = 0.25;
pub const DIFF_START_END_WEIGHT: f32 = 0.10;
pub const RANK_WEIGHT: f32 = 0.35;
pub const BOMB_WEIGHT: f32 = 0.15;
pub const LIFE_WEIGHT: f32 = 0.15;
