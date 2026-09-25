use crate::param::RuntimeConfig;
use serde_json::Value;

fn instruction(program_json: &str, event: &str) -> Option<Value> {
    serde_json::from_str::<Vec<Value>>(program_json)
        .unwrap()
        .into_iter()
        .find(|value| value.get("when").and_then(Value::as_str) == Some(event))
}

pub fn execute_cc_json(
    game: &str,
    program_json: &str,
    event: &str,
    current_json: Option<&str>,
) -> String {
    let Some(value) = instruction(program_json, event) else {
        return current_json.unwrap().to_string();
    };
    match game {
        "th05c" => crate::games::th05c::cfg::execute_cc_instruction(value, current_json),
        _ => unreachable!(),
    }
}

pub fn write_cfg_json(cfg: &RuntimeConfig, cfg_json: &str) {
    match cfg.runtime.game.as_str() {
        "th05c" => crate::games::th05c::cfg::write_runtime_cfg(cfg, cfg_json),
        _ => unreachable!(),
    }
}

pub fn debug_cfg_json(game: &str, cfg_json: &str) -> String {
    match game {
        "th05c" => serde_json::from_str::<crate::games::th05c::cfg::Cfg05>(cfg_json)
            .unwrap()
            .debug_str(),
        _ => unreachable!(),
    }
}
