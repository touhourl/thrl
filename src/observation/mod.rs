pub mod frame;
pub mod schema1;

use frame::Frame;
use std::sync::Arc;

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
