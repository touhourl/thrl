pub mod boss;
pub mod builder;
pub mod frame;
pub mod map;
pub mod player;
pub mod reward;
pub mod state;

pub use boss::*;
pub use builder::*;
pub use frame::*;
pub use map::*;
pub use player::*;
pub use reward::*;
pub use state::*;

/*
    Schema module of rrr.
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

use crate::observation::{EncodedState, SchemaSpec};
use std::sync::Arc;

pub fn spec() -> SchemaSpec {
    let builder = ObservationBuilder::default();
    SchemaSpec {
        observations: vec![
            vec![Observation::feature_only_count()],
            vec![
                Observation::map_channel_count(),
                builder.grid_h,
                builder.grid_w,
            ],
        ],
        rewards: 3,
    }
}

pub fn encode(prev: Option<&frame::Frame>, frame: &mut frame::Frame) -> EncodedState {
    frame.update_bomb_tracking(prev);
    let end = frame.end;
    let mut rewards = reward::calculate_reward_m(prev, frame);
    if end == 1 {
        rewards[0] -= 100.0;
    } else if end == 2 {
        rewards[0] += 100.0;
    }
    let (features, maps) = ObservationBuilder::default().build_components(frame);
    EncodedState {
        observations: vec![Arc::new(features), Arc::new(maps)],
        end,
        rewards,
    }
}
