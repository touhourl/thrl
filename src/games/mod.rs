pub mod common;
//pub mod th04;
pub mod th05c;
pub mod th05d;
// use crate::error::Result;
// use self::common::finder::GameFinder;
// It is called th05c because I used anniversery branch of ReC98
// project. For now, only th05 is present (the custom version), so
// I just map th05 to th05c.
//
// Create a finder for the specified game.
//pub fn create_finder(game: &str, pid: i32) -> Result<Box<dyn GameFinder>> {
//    match game {
//        //"th04" => Ok(Box::new(th04::finder::TH04Finder::new(pid)?)),
//        "th05" => Ok(Box::new(th05c::finder::TH05Finder::new(pid)?)),
//        _ => Err(crate::error::Error::InvalidConfig {
//            reason: format!("Unknown game: {}", game),
//        }),
//    }
//}

use crate::observation::EncodedState;
use crate::param::RuntimeConfig;
use std::time::Duration;

#[derive(Clone, serde::Serialize)]
pub struct EnvSpec {
    pub observations: Vec<Vec<usize>>,
    pub actions: usize,
    pub rewards: usize,
}

pub trait Environment: Send {
    fn initial_state(&mut self) -> Result<Option<EncodedState>, String>;
    fn step(
        &mut self,
        action: usize,
        frame_elapsed: Duration,
    ) -> Result<Option<EncodedState>, String>;
    fn reset(&mut self) -> Result<Option<EncodedState>, String>;
    fn human_action(&mut self) -> Result<Option<usize>, String> {
        Ok(None)
    }
}

pub fn spec(cfg: &RuntimeConfig) -> Result<EnvSpec, String> {
    match cfg.runtime.game.as_str() {
        "th05c" => {
            let schema = crate::observation::spec(cfg.runtime.schema)?;
            Ok(EnvSpec {
                observations: schema.observations,
                actions: th05c::key::DiscreteAction::size(),
                rewards: schema.rewards,
            })
        }
        game => Err(format!("Unsupported game worker: {game}")),
    }
}

pub fn spawn(cfg: &RuntimeConfig) -> Result<Box<dyn Environment>, String> {
    match cfg.runtime.game.as_str() {
        "th05c" => Ok(Box::new(th05c::watcher::TH05CSession::spawn(cfg)?)),
        game => Err(format!("Unsupported game worker: {game}")),
    }
}
