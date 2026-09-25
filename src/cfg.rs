use crate::param::RuntimeConfig;
use serde_json::Value;
/*
    General CC configuration of rrr.
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
