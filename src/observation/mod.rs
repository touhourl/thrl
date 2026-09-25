pub mod frame;
pub mod schema1;

use frame::Frame;
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

pub fn spec(schema: usize) -> Result<SchemaSpec, String> {
    match schema {
        1 => Ok(schema1::spec()),
        _ => Err(format!("Unsupported observation schema: {schema}")),
    }
}

pub fn encode(
    schema: usize,
    prev: Option<&Frame>,
    frame: &mut Frame,
) -> Result<EncodedState, String> {
    frame.update_bomb_tracking(prev);
    match schema {
        1 => Ok(schema1::encode(prev, frame)),
        _ => Err(format!("Unsupported observation schema: {schema}")),
    }
}
