pub mod common;
pub mod th04a;
//pub mod th04;
pub mod th05c;
pub mod th05d;

/*
    ENV of thrl.
    Copyright (C) 2026  T. Liu (touhourl@proton.me)

    This program is free software: you can redistribute it and/or modify
    it under the terms of the GNU Affero General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU Affero General Public License for more details.

    You should have received a copy of the GNU Affero General Public License
    along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/

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

use crate::observation::{Frame, Schema};
use crate::param::RuntimeConfig;
use std::time::Duration;

#[derive(Clone, serde::Serialize)]
pub struct EnvSpec {
    pub observations: Vec<Vec<usize>>,
    pub actions: usize,
    pub rewards: usize,
}

pub trait Environment: Send {
    fn initial_state(&mut self) -> Result<Option<Frame>, String>;
    fn step(&mut self, action: usize, frame_elapsed: Duration) -> Result<Option<Frame>, String>;
    fn reset(&mut self) -> Result<Option<Frame>, String>;
    fn human_action(&mut self) -> Result<Option<usize>, String> {
        Ok(None)
    }
}

pub fn spec(cfg: &RuntimeConfig) -> Result<EnvSpec, String> {
    match Schema::try_from(cfg.runtime.schema)? {
        Schema::Schema0 => {
            if cfg.runtime.game != "taisei" {
                return Err(format!(
                    "schema0 requires taisei, got {}",
                    cfg.runtime.game.as_str()
                ));
            }
            let schema = crate::observation::spec(cfg)?;
            Ok(EnvSpec {
                observations: schema.observations,
                actions: crate::observation::schema0::ACTION_COUNT,
                rewards: schema.rewards,
            })
        }
        Schema::Schema1 => match cfg.runtime.game.as_str() {
            "th04a" | "th05c" => {
                let schema = crate::observation::spec(cfg)?;
                Ok(EnvSpec {
                    observations: schema.observations,
                    actions: th05c::key::DiscreteAction::size(),
                    rewards: schema.rewards,
                })
            }
            game => Err(format!("Unsupported game worker: {game}")),
        },
    }
}

pub fn spawn(cfg: &RuntimeConfig) -> Result<Box<dyn Environment>, String> {
    match Schema::try_from(cfg.runtime.schema)? {
        Schema::Schema0 => {
            if cfg.runtime.game != "taisei" {
                return Err(format!(
                    "schema0 requires taisei, got {}",
                    cfg.runtime.game.as_str()
                ));
            }
            {
                Ok(Box::new(crate::observation::schema0::TaiseiSession::spawn(
                    cfg,
                )?))
            }
        }
        Schema::Schema1 => match cfg.runtime.game.as_str() {
            "th05c" => Ok(Box::new(th05c::watcher::TH05CSession::spawn(cfg)?)),
            "th04a" => Ok(Box::new(th04a::TH04ASession::spawn(cfg)?)),
            game => Err(format!("Unsupported game worker: {game}")),
        },
    }
}
