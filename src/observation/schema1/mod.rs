pub mod boss;
pub mod builder;
pub mod map;
pub mod player;
pub mod reward;
pub mod state;

pub use boss::*;
pub use builder::*;
pub use map::*;
pub use player::*;
pub use reward::*;
pub use state::*;

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

pub fn encode(
    prev: Option<&crate::observation::frame::Frame>,
    frame: &crate::observation::frame::Frame,
) -> EncodedState {
    let end = frame.resident.game_end_flag;
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
