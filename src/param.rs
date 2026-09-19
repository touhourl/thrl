//! This file provides compiled configs
//! so we just need to modify it while changing things
//! like game variable, logging, etc
//!


/*
    Parameters of rrr.
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
use crate::games::th05_c::key::DiscreteAction;
pub struct TrainingConfig;
impl TrainingConfig {
    pub const REWARD_SCALE: f32 = 0.01;
    pub const LEARNING_RATE: f32 = 1e-4;
    pub const GAMMA: f32 = 0.99;
    pub const LAMBDA: f32 = 0.97;
    pub const CLIP_EPSILON: f32 = 0.2;
    pub const VALUE_COEFF: f32 = 0.5;
    pub const ENTROPY_COEFF_START: f32 = 0.05;
    pub const ENTROPY_COEFF_END: f32 = 0.001;
    pub const ENTROPY_ANNEAL_UPDATES: usize = 3000;
    pub const PPO_EPOCHS: usize = 4;
    pub const MINI_BATCH_SIZE: usize = 64;
    pub const HORIZON: usize = 2048;
    pub const HIDDEN_DIM: usize = 512;
    pub const ACTION_DIM: usize = DiscreteAction::size();
    pub const ADVANTAGE_NORM_EPSILON: f32 = 1e-8;
    pub const PROB_EPSILON: f32 = 1e-8;
    pub const LOGIT_CLAMP: f32 = 80.0;
    pub const LOG_RATIO_CLAMP: f32 = 20.0;
    pub const FRAME_INTERVAL_MS: u64 = 53; // 3 Frame Skipping
    pub const MAX_GRAD_NORM: f32 = 0.5;
    pub const DEV: &str = "GPU";
    pub const CNN_HIDDEN_CHANNELS: [usize; 3] = [32, 64, 64];
    pub const CNN_EMBED_DIM: usize = 128;
    pub const CNN_POOL_OUT: [usize; 2] = [6, 6];
    pub const FEATURE_PROJECT_DIM: usize = 128;
    pub const GRU_HIDDEN_SIZE: usize = 256;
    pub const SEQ_LEN: usize = 16;
    /// Feature dim: player(17) + boss(14) + state(6) + projectile_entities(112) +
    /// bullet_entities(112) + drop(12) = 273
    pub const FEATURE_DIM: usize = 273;
    /// Map channels: bullet(6) + enemy(6) + projectile_merged(6) + boss(6) = 24
    pub const MAP_CHANNELS: usize = 24;

    pub fn entropy_coeff(update_step: usize) -> f32 {
        if Self::ENTROPY_ANNEAL_UPDATES == 0 {
            return Self::ENTROPY_COEFF_END;
        }

        let t = (update_step as f32 / Self::ENTROPY_ANNEAL_UPDATES as f32).min(1.0);
        Self::ENTROPY_COEFF_START + t * (Self::ENTROPY_COEFF_END - Self::ENTROPY_COEFF_START)
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
