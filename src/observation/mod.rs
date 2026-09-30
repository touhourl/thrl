pub mod frame;
pub mod schema1;

pub use frame::Frame;
use std::sync::Arc;

/*
    Observations of rrr.
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
    Schema1,
}

impl TryFrom<usize> for Schema {
    type Error = String;

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::Schema1),
            _ => Err(format!("Unsupported observation schema: {value}")),
        }
    }
}

pub fn spec(schema: usize) -> Result<SchemaSpec, String> {
    match Schema::try_from(schema)? {
        Schema::Schema1 => Ok(schema1::spec()),
    }
}

pub fn encode(prev: Option<&Frame>, frame: &mut Frame) -> Result<EncodedState, String> {
    match (prev, frame) {
        (None, Frame::Schema1(frame)) => Ok(schema1::encode(None, frame)),
        (Some(Frame::Schema1(prev)), Frame::Schema1(frame)) => {
            Ok(schema1::encode(Some(prev), frame))
        }
    }
}
