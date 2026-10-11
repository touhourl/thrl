pub mod frame;
pub mod schema0;
pub mod schema1;

use crate::param::RuntimeConfig;
pub use frame::Frame;
use std::sync::Arc;

/*
    Observations of thrl.
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
#[derive(Clone)]
pub struct EncodedState {
    pub observations: Vec<Arc<Vec<f32>>>,
    pub end: u8,
    pub rewards: Vec<f32>,
}

#[derive(Clone)]
pub struct SchemaSpec {
    pub observations: Vec<Vec<usize>>,
    pub rewards: usize,
}

#[derive(Clone, Copy)]
pub enum Schema {
    Schema0,
    Schema1,
}

impl TryFrom<usize> for Schema {
    type Error = String;

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Schema0),
            1 => Ok(Self::Schema1),
            _ => Err(format!("Unsupported observation schema: {value}")),
        }
    }
}

pub fn spec(cfg: &RuntimeConfig) -> Result<SchemaSpec, String> {
    match Schema::try_from(cfg.runtime.schema)? {
        Schema::Schema0 => Ok(schema0::spec(cfg)),
        Schema::Schema1 => Ok(schema1::spec()),
    }
}

pub fn encode(
    cfg: &RuntimeConfig,
    prev: Option<&Frame>,
    frame: &mut Frame,
) -> Result<EncodedState, String> {
    match (prev, frame) {
        (None, Frame::Schema0(frame)) => Ok(schema0::encode::encode(cfg, None, frame)),
        (Some(Frame::Schema0(prev)), Frame::Schema0(frame)) => {
            Ok(schema0::encode::encode(cfg, Some(prev), frame))
        }
        (None, Frame::Schema1(frame)) => Ok(schema1::encode(None, frame)),
        (Some(Frame::Schema1(prev)), Frame::Schema1(frame)) => {
            Ok(schema1::encode(Some(prev), frame))
        }
        _ => Err("Observation schema changed between frames".to_string()),
    }
}
