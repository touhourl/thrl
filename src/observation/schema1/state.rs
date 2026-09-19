use crate::games::th05_c::types::GameState;
use serde::{Deserialize, Serialize};

/*
    State features of rrr.
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateFeatures {
    pub stage_norm: f32,
    pub graze_norm: f32,
    pub score_norm: f32,
    pub miss_count_norm: f32,
    pub point_items_norm: f32,
    pub rank_norm: f32,
}

impl StateFeatures {
    pub fn from_game_state(state: &GameState) -> Self {
        Self {
            // 7 stages (6 in resident)
            stage_norm: (state.resident.stage as f32 / 6.0).clamp(0.0, 1.0),
            // game limited: actually only to 999.
            graze_norm: (state.stage_collection.stage_graze as f32 / 1000.0).clamp(0.0, 1.0),
            // no shit but not used.... since th05
            score_norm: (state.resident.score as f32 / 100_000_000.0).clamp(0.0, 1.0),
            // absolutely not more than 10, that's why the specific cfg gen i cannot do 99 lives
            miss_count_norm: (state.resident.miss_count as f32 / 10.0).clamp(0.0, 1.0),
            // I think it is reasonable, tho I have never collected so much.
            point_items_norm: (state.stage_collection.point_items_stage as f32 / 100.0)
                .clamp(0.0, 1.0),
            rank_norm: (state.resident.rank as f32 / 3.0).clamp(0.0, 1.0), //4 ranks
        }
    }

    pub fn to_vec(&self) -> Vec<f32> {
        vec![
            self.stage_norm,
            self.graze_norm,
            self.score_norm,
            self.miss_count_norm,
            self.point_items_norm,
            self.rank_norm,
        ]
    }
    // dumb fn can i delete you
    pub const fn feature_count() -> usize {
        6
    }
}
