use crate::observation::frame::Frame;
use serde::{Deserialize, Serialize};

/*
    Player Observation of rrr.
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
// Thanks ReC98 for the value and magic number!
const MIN_X: f32 = 8.0;
const MAX_X: f32 = 376.0;
const MIN_Y: f32 = 8.0;
const MAX_Y: f32 = 352.0;
const RANGE_X: f32 = MAX_X - MIN_X;
const RANGE_Y: f32 = MAX_Y - MIN_Y;
const POWER_MAX: f32 = 128.0;
const VELOCITY_NORM_DIVISOR: f32 = 12.0;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerFeatures {
    pub x_norm: f32,
    pub y_norm: f32,
    pub vx_norm: f32,
    pub vy_norm: f32,
    pub wall_left: f32,
    pub wall_right: f32,
    pub wall_top: f32,
    pub wall_bottom: f32,
    pub power_norm: f32,
    pub lives_norm: f32,
    pub bombs_norm: f32,
    pub invincible: f32,
    pub character_norm: f32,
    pub stage_norm: f32,
    pub cfg_lives_norm: f32,
    pub cfg_bombs_norm: f32,
    pub rank_norm: f32,
}

impl PlayerFeatures {
    pub fn from_game_state(state: &Frame) -> Self {
        let (px, py) = state.player.pos.to_pixels();
        let (vx, vy) = state.player.pos.velocity_pixels();
        let x_norm = ((px - MIN_X) / RANGE_X).clamp(0.0, 1.0);
        let y_norm = ((py - MIN_Y) / RANGE_Y).clamp(0.0, 1.0);
        let vx_norm = (vx / VELOCITY_NORM_DIVISOR).clamp(-1.0, 1.0);
        let vy_norm = (vy / VELOCITY_NORM_DIVISOR).clamp(-1.0, 1.0);

        let walls = wall_distances(px, py);

        let power_norm = (state.player.power as f32 / POWER_MAX).clamp(0.0, 1.0);
        let lives_norm = (state.resident.rem_lives as f32 / 8.0).clamp(0.0, 1.0);
        let bombs_norm = (state.rem_bombs_internal as f32 / 8.0).clamp(0.0, 1.0);

        let invincible = if state.player.invincibility_time > 0
            || state.player.invincible_via_bomb
            || state.player.miss_frame > 0
        {
            1.0
        } else {
            0.0
        };

        let character_norm = (state.resident.playchar as f32 / 3.0).clamp(0.0, 1.0);
        let stage_norm = (state.resident.stage as f32 / 6.0).clamp(0.0, 1.0);
        let cfg_lives_norm = (state.resident.credit_lives as f32 / 8.0).clamp(0.0, 1.0);
        let cfg_bombs_norm = (state.resident.credit_bombs as f32 / 8.0).clamp(0.0, 1.0);
        let rank_norm = (state.resident.rank as f32 / 3.0).clamp(0.0, 1.0);

        Self {
            x_norm,
            y_norm,
            vx_norm,
            vy_norm,
            wall_left: walls[0],
            wall_right: walls[1],
            wall_top: walls[2],
            wall_bottom: walls[3],
            power_norm,
            lives_norm,
            bombs_norm,
            invincible,
            character_norm,
            stage_norm,
            cfg_lives_norm,
            cfg_bombs_norm,
            rank_norm,
        }
    }

    pub fn to_vec(&self) -> Vec<f32> {
        vec![
            self.x_norm,
            self.y_norm,
            self.vx_norm,
            self.vy_norm,
            self.wall_left,
            self.wall_right,
            self.wall_top,
            self.wall_bottom,
            self.power_norm,
            self.lives_norm,
            self.bombs_norm,
            self.invincible,
            self.character_norm,
            self.stage_norm,
            self.cfg_lives_norm,
            self.cfg_bombs_norm,
            self.rank_norm,
        ]
    }

    // can I delete u get out...
    pub const fn feature_count() -> usize {
        17
    }
}

/// Is only used once but... might be used again in the future?
/// The one in reward.rs cannot be used here because I devide there. here I don't.
fn wall_distances(px: f32, py: f32) -> [f32; 4] {
    let left = (px - MIN_X).max(0.0);
    let right = (MAX_X - px).max(0.0);
    let top = (py - MIN_Y).max(0.0);
    let bottom = (MAX_Y - py).max(0.0);

    [
        (left / RANGE_X).clamp(0.0, 1.0),
        (right / RANGE_X).clamp(0.0, 1.0),
        (top / RANGE_Y).clamp(0.0, 1.0),
        (bottom / RANGE_Y).clamp(0.0, 1.0),
    ]
}
