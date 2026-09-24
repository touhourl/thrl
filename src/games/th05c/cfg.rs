//! This file contains tested working configs of th05.
//!
//! Because I don't have so much time, I won't use the .json
//! or .ini or even .cfg file to change them.
//! I will make rules here, not burte-force of config
//! It will be compiled because they should be unique
//! and is in debloatm determinestic.
//!
//! IMPORTANT: Bomb and lives CANNOT be over 3.
//!
//! Can someone play game for me and test out combinations...
//! I am terrible....
//!
//! [control]
//! [feature]
//!
//! In the config the value of 1 and 2 are literally the same,
//! so setting (1, 2) will just do once and nothing.
//! The only difference that is useful and add the difficulty is,
//! that 1 starts before the dialog transition and 2 starts after the
//! dialog transation. I am now too lazy to adjust it again...
//!
//! After freeze: this is only for th05. Th06... hard and might need further
//! works but lemme donau the MORL farst...

/*
    Curriculum Algorithms of rrr.
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
use crate::param;
use rand::RngExt;
use serde::Deserialize;
use serde_json::{self, Value};
use std::io::{self, Write};
use std::path::Path;
/// In KAIKII.CFG:
/// total_live=             ; The lives, (cfg_live and live)
/// total_bomb=             ; The bombs, (cfg_bomb abd bomb)
/// skip_to=                ; Skip to which stage
/// skip_to_boss_phase=     ; Phase:
/// end_phase=              ; End phase. Logic same as `skip_to_boss_phase`
/// char=                   ; Characters.
///                         ; PLAYCHAR_REIMU = 0
///                         ; PLAYCHAR_MARISA = 1
///                         ; PLAYCHAR_MIMA = 2
///                         ; PLAYCHAR_YUUKA = 3
/// rank=                   ; 0 = Easy, 1 = Normal, 2 = Hard, 3 = Lunatic
/// power=                  ; Power, 0 - 128.
///                         ; We can't lose power if we don't die
/// Just keep all things u8 because rust don't allow u4
/// IMPORTANT: Bomb and lives CANNOT be over 3.
/// You have been warned.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, Deserialize)]
pub struct Cfg05 {
    pub live: u8,
    pub bomb: u8,
    pub stg: u8,
    pub phase: u8,
    pub end: u8,
    // [conflict]
    pub cha: u8,
    pub rank: u8,
    pub power: u8,
}

impl Cfg05 {
    pub fn debug_str(&self) -> String {
        format!(
            "cfg05{{lives:{}, bombs:{}, stage:{}, char:{}, rank:{}, power:{}}}",
            self.live, self.bomb, self.stg, self.cha, self.rank, self.power
        )
    }

    pub fn json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
}

impl Default for Cfg05 {
    fn default() -> Self {
        Cfg05 {
            live: 3,
            bomb: 3,
            stg: 0,
            phase: 0,
            end: 0,
            cha: 0,
            rank: 0,
            power: 0,
        }
    }
}

#[derive(Debug, Deserialize)]
struct InstHeader {
    #[serde(rename = "type")]
    kind: String,
    name: String,
}
// Denying them multiple times. Why don't you read docs? Oh shit I forgot to
// write docs but please wait, wait, wait my taisei-sim complete pls...
#[derive(Debug, Deserialize)]
struct StructInst {
    live: u8,
    bomb: u8,
    stg: u8,
    phase: u8,
    end: u8,
    cha: u8,
    rank: u8,
    power: u8,
}

#[derive(Debug, Deserialize)]
struct SpecificCfgGenInst {
    stage: u8,
    phase: u8,
    rank: u8,
    cha: u8,
}

#[derive(Debug, Deserialize)]
struct CfgGenInst {
    stage: u8,
    char_pool: Vec<u8>,
    rank_min: u8,
    rank_max: u8,
    live: u8,
    bomb: u8,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum PlayperfScore {
    Fixed(u8),
    Relative { from: ScoreSource, delta: i16 },
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ScoreSource {
    CurrentConfig,
}

#[derive(Debug, Deserialize)]
struct PlayperfInst {
    score: PlayperfScore,
    tolerance: u8,
    time_ms: u64,
    char_pool: Vec<u8>,
}

/// Parse the playperf score and do plus/minus, so I don't need to write it in shitty python
fn resolve_playperf_score(score: &PlayperfScore, current: Option<Cfg05>) -> u8 {
    match score {
        PlayperfScore::Fixed(score) => *score,
        PlayperfScore::Relative {
            from: ScoreSource::CurrentConfig,
            delta,
        } => {
            let current = current.unwrap();
            // Plus/minus
            let current_score = score_cfg(current).score as i16;
            (current_score + delta) as u8
        }
    }
}

/// We match the json here. I don't want to see some eval() code in rust,
/// so use switch and match.
pub fn execute_cc_instruction(value: Value, current_json: Option<&str>) -> String {
    let header: InstHeader = serde_json::from_value(value.clone()).unwrap();
    let current = current_json.map(|json| serde_json::from_str::<Cfg05>(json).unwrap());
    let cfg = match (header.kind.as_str(), header.name.as_str()) {
        ("struct", "Cfg05") => {
            let spec: StructInst = serde_json::from_value(value).unwrap();
            Cfg05 {
                live: spec.live,
                bomb: spec.bomb,
                stg: spec.stg,
                phase: spec.phase,
                end: spec.end,
                cha: spec.cha,
                rank: spec.rank,
                power: spec.power,
            }
        }
        ("fn", "specific_cfg_gen") => {
            let spec: SpecificCfgGenInst = serde_json::from_value(value).unwrap();
            specific_cfg_gen(spec.stage, spec.phase, spec.rank, spec.cha)
        }
        ("fn", "cfg_gen") => {
            let spec: CfgGenInst = serde_json::from_value(value).unwrap();
            cfg_gen(
                spec.stage,
                &spec.char_pool,
                (spec.rank_min, spec.rank_max),
                spec.live,
                spec.bomb,
            )
            .unwrap()
        }
        ("fn", "playperf") => {
            let spec: PlayperfInst = serde_json::from_value(value).unwrap();
            let score = resolve_playperf_score(&spec.score, current);
            let configs = playperf(score, spec.tolerance, spec.time_ms, &spec.char_pool);
            configs[rand::rng().random_range(0..configs.len())].cfg
        }
        ("advance", "specific_cfg_gen") => {
            let current = current.unwrap();
            specific_cfg_gen(current.stg, current.phase, current.rank, current.cha)
        }
        _ => unreachable!(),
    };
    serde_json::to_string(&cfg).unwrap()
}

/// Decode and write a json-Cfg05
pub fn write_cfg_json(path: &Path, cfg_json: &str) {
    let cfg = serde_json::from_str(cfg_json).unwrap();
    cfg_write(path, &cfg).unwrap();
}

pub fn write_runtime_cfg(cfg: &crate::param::RuntimeConfig, cfg_json: &str) {
    let dir = Path::new(cfg.raw["paths"]["export_dir"].as_str().unwrap());
    std::fs::create_dir_all(dir).unwrap();
    let path = dir.join(cfg.raw["paths"]["cfg_file"].as_str().unwrap());
    write_cfg_json(&path, cfg_json);
}

/// This struct is to rate the cfgs
/// according to different configs
/// so the agent gets a score according to
/// it's last or training phase performance.
/// We could efficiently use the score because
/// u8 is limited to 255 which we can use.
/// IMPORTANT: Bomb and lives CANNOT be over 3.
#[derive(Default)]
pub struct Returncfg {
    pub cfg: Cfg05,
    pub score: u8,
}

// Somehow useless for compiler but useful for us.
// Why not in const.rs? Because it is not used.
//
// [copy]
//
// Stage 1 Boss - Sara
// Stage 6 Boss - Shinki
//
// [copy]

/// This is the all cases that ends with 0 (which will somehow always work)
/// But we need to define the internal_level there.
/// So, not quite easy btw.
///
/// The format: `(stage, skip_to_phase, end_in_phase)`
///
/// As stated, the forbidden combos are:
///
/// 1. (x, 1, 2)
///
/// Reason:
/// In the config the value of 1 and 2 are literally the same,
/// so setting (1, 2) will just do once and nothing.
/// The only difference that is useful and add the difficulty is,
/// that 1 starts before the dialog transition and 2 starts after the
/// dialog transation. I am now too lazy to adjust it again...
/// They will just waste 5 seconds.
///
/// 2. (1, 3, 4) (1, 4, 5)
///
/// Reason: Stage 2. Too short (bug?) It has the same HP at 4 and 5.
///
/// 3. (2, 3, 4) (2, 4, 5) (2, 6, 7) (2, 7, 8) (2, 9, 10) (2, 10, 11) (2, 11, 12)
///
/// Reason: Are just repeats.
///
/// 4. IMPORTANT: Only those are **accepted**, not **rejected**
///
/// (3, 0, 1) (3, 0, 0) (3, 2, 0) (3, 3, 0)
///
/// 5. (4, 3, 4) (4, 4, 5) (4, 6, 7) (4, 7, 8)
///
/// Reason: Same
///
/// 6. (5, 4, 5) (5, 5, 6) (5, 5, 7), (5, 6, 7)
///
/// 7. (6, 3, 4) (6, 4, 5) (6, 4, 6) (6, 5, 6) (6, 6, 7) (6, 6, 8) (6, 6, 7)
///    (6, 7, 8) (6, 8, 9) (6, 8, 10) (6, 8, 7) (6, 9, 10) (6, 10, 11)
///    (6, 10, 12) (6, 11, 12) (6, 12, 13) (6, 12, 14) (6, 13, 14)
///
/// Reason: Tested and same for Stage 6 and 7.
///
/// Rule 1: Any value of skip_to_phase cannot exceed `fn selectable_phase(stage)`
///
/// Rule 2: Any value of end_in_phase cannot exceed `fn max_phase(stage)`
///
/// Rule 3: skip_to_phase cannot be equal or lower than the end_in_phase. Does not apply
/// to full stage run.
///
/// Rule 4: Never have something of (x, x, 2) or (x, 1, x) - not time-optimal
const FORBIDDEN_COMBOS: &[(u8, u8, u8)] = &[
    (1, 3, 4),
    (1, 4, 5),
    (2, 3, 4),
    (2, 4, 5),
    (2, 6, 7),
    (2, 7, 8),
    (2, 9, 10),
    (2, 10, 11),
    (2, 11, 12),
    (4, 3, 4),
    (4, 4, 5),
    (4, 6, 7),
    (4, 7, 8),
    (5, 4, 5),
    (5, 5, 6),
    (5, 5, 7),
    (5, 6, 7),
    (6, 3, 4),
    (6, 4, 5),
    (6, 4, 6),
    (6, 5, 6),
    (6, 6, 7),
    (6, 6, 8),
    (6, 7, 8),
    (6, 8, 9),
    (6, 8, 10),
    (6, 9, 10),
    (6, 10, 11),
    (6, 10, 12),
    (6, 11, 12),
    (6, 12, 13),
    (6, 12, 14),
    (6, 13, 14),
];

fn is_valid_combo(stage: u8, skip_to_phase: u8, end_in_phase: u8) -> bool {
    // Phase 1 is before dialog and 2 is after dialog.
    // We rather waste 1k instructions here than to have 5s delay of boss HP.
    if skip_to_phase == 1 || end_in_phase == 2 {
        return false;
    }

    // Stage 3 only, whitelist.
    if stage == 3 {
        return matches!(
            (stage, skip_to_phase, end_in_phase),
            (3, 0, 1) | (3, 0, 0) | (3, 2, 0) | (3, 3, 0)
        );
    }

    // Rule 1: skip_to_phase cannot exceed selectable_phase
    if skip_to_phase > selectable_phase(stage) {
        return false;
    }

    // Rule 2: end_in_phase cannot exceed max_phase
    if end_in_phase > max_phase(stage) {
        return false;
    }

    // Design check: is the end 0? Tbat might be a full stage run... Good luck!
    if end_in_phase == 0 {
        return true;
    }

    // Rule 3: end_in_phase MUST be > skip_to_phase
    if end_in_phase <= skip_to_phase {
        return false;
    }

    // All other stages: reject if in forbidden list
    !FORBIDDEN_COMBOS.contains(&(stage, skip_to_phase, end_in_phase))
}

/// Power level assignment based on stage and skip position.
///
/// Fresh starts (skip_to == 0) have lower power because the player
/// will naturally accumulate items through the stage, if they don't die
/// enough times or inting. Boss skips (skip_to != 0) get higher minimum power.
///
/// Stage 0 and 6 are reset to power 0 on fresh start because they are
/// start stages: Stage 0 is the game beginning, Stage 6 (Extra) is a
/// complete standalone run. They won't have ++ power.
///
/// Stage 6 boss skip, if you skip_to != 0 in Stage 6, you MUST have 96+
/// power minimum. The final boss, ExAlice is lunatic level already and the player
/// will get sudo kill -9 if power is below this.
///
/// As stage increases, power range shifts higher, I recommand the player to
/// learn collecting power items.
pub fn cfg_gen(
    stage: u8,
    char_pool: &[u8],
    rank_range: (u8, u8),
    live: u8,
    bomb: u8,
) -> Option<Cfg05> {
    assert!(stage < 7, "Invalid stage");
    assert!(!char_pool.is_empty(), "Character pool cannot be empty");
    assert!(
        char_pool.iter().all(|&c| c <= 3),
        "Invalid character ID in pool"
    );
    assert!(rank_range.0 <= 3 && rank_range.1 <= 3, "Invalid rank range");
    assert!(live <= 3 && bomb <= 3, "Lives and bombs must be <= 3");
    assert!(
        rank_range.0 <= rank_range.1,
        "Invalid rank range: start > end"
    );
    let mut rng = rand::rng();
    // Find all valid (phase, end) pairs for this stage
    let mut valid_pairs = Vec::new();

    for skip_to in 0..=selectable_phase(stage) {
        for end in 0..=max_phase(stage) {
            if is_valid_combo(stage, skip_to, end) {
                valid_pairs.push((skip_to, end));
            }
        }
    }

    if valid_pairs.is_empty() {
        return None;
    }

    let (skip_to, end) = valid_pairs[rng.random_range(0..valid_pairs.len())];
    let char_id = char_pool[rng.random_range(0..char_pool.len())];
    let rank = rng.random_range(rank_range.0..=rank_range.1);
    let power: u8 = get_power(skip_to, stage);
    Some(Cfg05 {
        live,
        bomb,
        stg: stage,
        phase: skip_to,
        end,
        cha: char_id,
        rank,
        power,
    })
}

/// This file is to generate a score for the config.
pub fn score_cfg(cfg: Cfg05) -> Returncfg {
    // Underflow detect (multiply by negative numbers, you have been warned)
    assert!(cfg.bomb <= 3, "Bomb must be <= 3");
    assert!(cfg.live <= 3, "Life must be <= 3");

    let stage_factor = (cfg.stg as f32) / 6.0;
    let skill_rank = if cfg.stg == 6 { 3 } else { cfg.rank };
    let mut returncfg = Returncfg::default();
    // Update the Returncfg with the one we recieved
    returncfg.cfg = cfg;
    let end: u8 = if cfg.end == 0 {
        max_phase(cfg.stg) + 1
    } else {
        cfg.end
    };
    let span = (end - cfg.phase) as f32;
    let span_factor = span / 15.0; // max_phase(6) + 1
    let rank_factor = (skill_rank as f32) / 3.0;
    let bomb_factor = (4 - cfg.bomb) as f32 / 4.0;
    let life_factor = (4 - cfg.live) as f32 / 4.0;
    let score: f32 = (stage_factor * param::STAGE_WEIGHT
        + span_factor * param::DIFF_START_END_WEIGHT
        + rank_factor * param::RANK_WEIGHT
        + bomb_factor * param::BOMB_WEIGHT
        + life_factor * param::LIFE_WEIGHT)
        * 255.0;
    returncfg.score = score.clamp(0.0, 255.0) as u8;
    returncfg
}

pub fn playperf(score: u8, tolerance: u8, time_ms: u64, char_pool: &[u8]) -> Vec<Returncfg> {
    let mut rng = rand::rng();
    let mut results = Vec::new();
    let start = std::time::Instant::now();
    let tolerance_i16 = tolerance as i16;
    let target_i16 = score as i16;

    while start.elapsed().as_millis() < time_ms as u128 {
        // Randomize all parameters
        let stage = rng.random_range(0..=6);

        // Random rank range [a, b]
        let rank_a = rng.random_range(0..=3);
        let rank_b = rng.random_range(rank_a..=3);
        let rank_range = (rank_a, rank_b);

        let live = rng.random_range(0..=3);
        let bomb = rng.random_range(0..=3);

        // Generate config with random parameters
        if let Some(cfg) = cfg_gen(stage, char_pool, rank_range, live, bomb) {
            let returncfg = score_cfg(cfg);
            let cfg_score = returncfg.score as i16;

            // Accept if within tolerance
            if (cfg_score - target_i16).abs() <= tolerance_i16 {
                results.push(returncfg);
            }
        }
    }
    results
}

#[inline]
fn get_power(skip_to: u8, stage: u8) -> u8 {
    let mut rng = rand::rng();
    let power: u8 = if skip_to == 0 {
        match stage {
            0 => 0,
            1 => rng.random_range(32..=64),
            2 => rng.random_range(48..=80),
            3 => rng.random_range(64..=96),
            4 => rng.random_range(64..=112),
            5 => rng.random_range(64..=128),
            6 => 0,
            _ => panic!("Too many stages!"),
        }
    } else {
        match stage {
            0 => rng.random_range(32..=64),
            1 => rng.random_range(48..=80),
            2 => rng.random_range(64..=96),
            3 => rng.random_range(80..=96),
            4 => rng.random_range(80..=112),
            5 => rng.random_range(96..=128),
            6 => rng.random_range(96..=128),
            _ => panic!("Too many stages!"),
        }
    };
    power
}
#[inline]
/// A dumb hard coded function for the maximum avalible value of the stage.
/// Returns the number of phases available for the given stage.
/// As state and tested:
/// Stage 1: 4 Phases
/// Stage 2: 6 Phases (because ZUN's code error? It has the same HP at 4 and 5)
/// Stage 3: 12 Phases (So many and some of them are just repeats)
/// Stage 4: [bug] We don't know how many phases...
///          For 2, it will be 2 person and will finish by the 2 person if end >= 4
///          For 3, it will skip to single person and there is **NO** valid end except for 1.
///          Just take it as 8 Phases
/// Stage 5: 9 Phases
/// Stage 6: 12 Phases
/// Extra Stage: To balance the difficulty of last 2 stages I set it as 14
/// E.g., max_phase(0) returns 4, meaning phases 0-4 are valid (5 phases), so you might need to +1
fn max_phase(stage: u8) -> u8 {
    // Just panic here because it is my error. No one else can make this panic
    // other than a programmer.
    assert!((stage as usize) < 7, "Invalid stage: {}", stage); // ex
    match stage {
        0 => 4,
        1 => 6,
        2 => 12,
        3 => 8,
        4 => 9,
        5 => 12,
        6 => 14,
        _ => unreachable!(),
    }
}

#[inline]
/// A dumb hard coded function for the maximum selectable value of the stage.
/// Returns the number of phases *selectable* for the given stage.
/// As state and tested:
/// Stage 1: 4 Phases
/// Stage 2: 6 Phases (because ZUN's code error? It has the same HP at 4 and 5)
/// Stage 3: 12 Phases (So many and some of them are just repeats)
/// Stage 4: [bug] We don't know how many phases...
///          For 2, it will be 2 person and will finish by the 2 person if end >= 4
///          For 3, it will skip to single person and there is **NO** valid end except for 1.s
///          Just take it as 3 Phases
/// Stage 5: 9 Phases
/// Stage 6: 6 Phases because ZUN change the background when > 7 or 8 I forgot.
/// Extra Stage: 12 Phases. Last 2 are unselectable we don't know why.
fn selectable_phase(stage: u8) -> u8 {
    assert!((stage as usize) < 7, "Invalid stage: {}", stage); // ex
    match stage {
        0 => 4,
        1 => 6,
        2 => 12,
        3 => 3,
        4 => 9,
        5 => 5,
        6 => 12,
        _ => unreachable!(),
    }
}

pub fn cfg_read(path: &Path) -> io::Result<Cfg05> {
    let content = std::fs::read_to_string(path)?;

    let mut cfg = Cfg05::default();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim();
            let value = value.trim();

            match key {
                "total_live" => cfg.live = value.parse().unwrap_or(2),
                "total_bomb" => cfg.bomb = value.parse().unwrap_or(3),
                "skip_to" => cfg.stg = value.parse().unwrap_or(0),
                "skip_to_boss_phase" => cfg.phase = value.parse().unwrap_or(0),
                "end_phase" => cfg.end = value.parse().unwrap_or(0),
                "char" => cfg.cha = value.parse().unwrap_or(0),
                "rank" => cfg.rank = value.parse().unwrap_or(0),
                "power" => cfg.power = value.parse().unwrap_or(0),
                _ => {}
            }
        }
    }

    Ok(cfg)
}

pub fn cfg_write(path: &Path, cfg: &Cfg05) -> io::Result<()> {
    let content = format!(
        "total_live={}\n\
         total_bomb={}\n\
         skip_to={}\n\
         skip_to_boss_phase={}\n\
         end_phase={}\n\
         char={}\n\
         rank={}\n\
         power={}\n",
        cfg.live, cfg.bomb, cfg.stg, cfg.phase, cfg.end, cfg.cha, cfg.rank, cfg.power
    );

    let mut file = std::fs::File::create(path)?;
    file.write_all(content.as_bytes())?;
    file.flush()?;

    Ok(())
}

#[inline]
/// Important: ONLY call it if the stage is successful. Otherwise, use
/// your last config.
/// This is a thing for unlimited death but no bombs.
/// Is normal stage operation.
/// The call of this function is to avoid the agent, never gonna give you up
/// never gonna let you down
/// so it cannot gameover. If gameover then, it will restart the current stage.
/// The stage is the current stage, mean by the last stage when we are requesting
/// new stages. success and the stage will + 1.
/// For stage 4 we need the phase.
pub fn specific_cfg_gen(stage: u8, phase: u8, rank: u8, cha: u8) -> Cfg05 {
    let mut cfg = Cfg05::default();
    cfg.rank = rank;
    cfg.cha = cha;
    if stage == 3 {
        if phase == 0 {
            cfg.stg = 3;
            cfg.phase = 3;
            cfg.end = 0;
        } else {
            cfg.stg = 4;
            cfg.phase = 0;
            cfg.end = 0;
        }
    } else if stage == 6 {
        cfg.rank = if rank == 3 { 0 } else { rank + 1 };
        cfg.stg = 0;
        cfg.phase = 0;
    } else {
        cfg.stg = stage + 1; // if success + 1
        cfg.end = 0;
        cfg.phase = 0;
    }
    cfg.power = get_power(cfg.phase, cfg.stg);
    cfg.bomb = 3;
    cfg.live = 3;
    cfg
}
