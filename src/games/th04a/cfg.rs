//! This file contains tested working configs of th04A.
//!
//! IMPORTANT: Bomb and lives CANNOT be over 3.
//!
//! ok morl is done : )

/*
    Curriculum Algorithm (th04A) of thrl.
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

/// In GENSOO.CFG:
/// total_live=             ; The lives, (cfg_live and live)
/// total_bomb=             ; The bombs, (cfg_bomb abd bomb)
/// skip_to=                ; Skip to which stage
/// skip_to_boss_phase=     ; Phase:
/// end_phase=              ; End phase. Logic same as `skip_to_boss_phase`
/// char=                   ; Characters.
///                         ; PLAYCHAR_REIMU and shottype = 0
///                         ; PLAYCHAR_REIMU and shottype = 1
///                         ; PLAYCHAR_MARISA and shottype = 0
///                         ; PLAYCHAR_MARISA and shottype = 1
/// rank=                   ; 0 = Easy, 1 = Normal, 2 = Hard, 3 = Lunatic
/// power=                  ; Power, 0 - 128.
///                         ; We can't lose power if we don't die
/// Just keep all things u8 because rust don't allow u4
/// IMPORTANT: Bomb and lives CANNOT be over 3.
/// You have been warned.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, Deserialize)]
pub struct Cfg04a {
    pub live: u8,
    pub bomb: u8,
    pub stg: u8,
    pub phase: u8,
    pub end: u8,
    pub cha: u8,
    pub rank: u8,
    pub power: u8,
}

impl Cfg04a {
    pub fn debug_str(&self) -> String {
        format!(
            "Cfg04a{{lives:{}, bombs:{}, stage:{}, char:{}, rank:{}, power:{}}}",
            self.live, self.bomb, self.stg, self.cha, self.rank, self.power
        )
    }

    pub fn json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
}

impl Default for Cfg04a {
    fn default() -> Self {
        Cfg04a {
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
fn resolve_playperf_score(score: &PlayperfScore, current: Option<Cfg04a>) -> u8 {
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
    let current = current_json.map(|json| serde_json::from_str::<Cfg04a>(json).unwrap());
    let cfg = match (header.kind.as_str(), header.name.as_str()) {
        ("struct", "Cfg04a") => {
            let spec: StructInst = serde_json::from_value(value).unwrap();
            Cfg04a {
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

/// Decode and write a json-Cfg04a
pub fn write_cfg_json(path: &Path, cfg_json: &str) {
    let cfg = serde_json::from_str(cfg_json).unwrap();
    cfg_write(path, &cfg).unwrap();
}

pub fn write_runtime_cfg(cfg: &crate::param::RuntimeConfig, cfg_json: &str) {
    let dir = &cfg.paths.export_dir;
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
    pub cfg: Cfg04a,
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

fn is_valid_combo(stage: u8, char: u8, skip_to_phase: u8, end_in_phase: u8) -> bool {
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
    if skip_to_phase > selectable_phase(stage, char) {
        return false;
    }

    // Rule 2: end_in_phase cannot exceed max_phase
    if end_in_phase > max_phase(stage, char) {
        return false;
    }

    // Design check: is the end 0? Tbat might be a full stage run... Good luck!
    if end_in_phase == 0 {
        return true;
    }

    // Rule 3: the margin between end and start must be 2 or more.
    if end_in_phase < skip_to_phase + 2 {
        return false;
    }

    true
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
) -> Option<Cfg04a> {
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
    let char = char_pool[rng.random_range(0..char_pool.len())];
    // Find all valid (phase, end) pairs for this stage
    let mut valid_pairs = Vec::new();

    for skip_to in 0..=selectable_phase(stage, char) {
        for end in 0..=max_phase(stage, char) {
            if is_valid_combo(stage, char, skip_to, end) {
                valid_pairs.push((skip_to, end));
            }
        }
    }

    if valid_pairs.is_empty() {
        return None;
    }

    let (skip_to, end) = valid_pairs[rng.random_range(0..valid_pairs.len())];
    let char_id = char;
    let rank = rng.random_range(rank_range.0..=rank_range.1);
    let power: u8 = get_power(skip_to, stage);
    Some(Cfg04a {
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
pub fn score_cfg(cfg: Cfg04a) -> Returncfg {
    // Underflow detect (multiply by negative numbers, you have been warned)
    assert!(cfg.bomb <= 3, "Bomb must be <= 3");
    assert!(cfg.live <= 3, "Life must be <= 3");

    let stage_factor = (cfg.stg as f32) / 6.0;
    let skill_rank = if cfg.stg == 6 { 3 } else { cfg.rank };
    let mut returncfg = Returncfg::default();
    // Update the Returncfg with the one we recieved
    returncfg.cfg = cfg;
    let end: u8 = if cfg.end == 0 {
        max_phase(cfg.stg, cfg.cha) + 1
    } else {
        cfg.end
    };
    let span = (end - cfg.phase) as f32;
    let span_factor = span / 8.0; // max_phase(6) + 1
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
///```asm
/// 		lesbx, _resident
/// 		moval, es:[bx+resident_t.end_phase]
/// 		oral, al
/// 		jznear ptr @@end_phase_done
/// 		cmp_stage_id, 2
/// 		jnzshort @@end_phase_not_elly
///
/// 		; Elly phase 1 is the floor attack
/// 		cmpal, 1
/// 		jbenear ptr @@end_phase_compare
/// 		cmp_boss_phase, 3
/// 		jbnear ptr @@end_phase_done
/// 		janear ptr @@end_phase_reached
/// 		subal, 2
/// 		cmp_elly_phase, al
/// 		jbnear ptr @@end_phase_done
/// 		jmp@@end_phase_reached
///
/// @@end_phase_not_elly:
/// 		cmp_stage_id, STAGE_EXTRA
/// 		jnzshort @@end_phase_stage_map
///
/// 		cmp_boss_statebyte[0].BSB_gengetsu_started, 0
/// 		jnzshort @@end_phase_gengetsu
///
/// 		; Mugetsu connected phases 1..2 map to raw attacks 2 and 4.
/// 		cmpal, 2
/// 		janear ptr @@end_phase_done
/// 		shlal, 1
/// 		jmp@@end_phase_compare
///
/// @@end_phase_gengetsu:
/// 		; Raw phase 6 is a transition
/// 		cmpal, 3
/// 		jbnear ptr @@end_phase_reached
/// 		cmpal, 6
/// 		janear ptr @@end_phase_compare
/// 		decal
/// 		jmp@@end_phase_compare
///
/// @@end_phase_stage_map:
/// 		cmp_stage_id, 0
/// 		jnzshort @@end_phase_stage1
/// 		incal
/// 		jmp@@end_phase_compare
///
/// @@end_phase_stage1:
/// 		cmp_stage_id, 1
/// 		jnzshort @@end_phase_stage3
/// 		incal
/// 		jmp@@end_phase_compare
///
/// @@end_phase_stage3:
/// 		cmp_stage_id, 3
/// 		jnzshort @@end_phase_stage4
/// 		cmp_playchar, PLAYCHAR_REIMU
/// 		jzshort @@end_phase_marisa
/// 		decal
/// 		pushbx
/// 		movah, 0
/// 		movbx, ax
/// 		moval, cs:@@end_phase_reimu_map[bx]
/// 		popbx
/// 		jmp@@end_phase_compare
///
/// @@end_phase_marisa:
/// 		cmp_boss_phase, 2
/// 		jbnear ptr @@end_phase_done
/// 		janear ptr @@end_phase_reached
/// 		decal
/// 		cmp_marisa_phase, al
/// 		jbnear ptr @@end_phase_done
/// 		jmp@@end_phase_reached
///
/// @@end_phase_stage4:
/// 		cmp_stage_id, 4
/// 		jnzshort @@end_phase_stage5
/// 		decal
/// 		pushbx
/// 		movah, 0
/// 		movbx, ax
/// 		moval, cs:@@end_phase_stage4_map[bx]
/// 		popbx
/// 		jmp@@end_phase_compare
///
/// @@end_phase_stage5:
/// 		decal
/// 		pushbx
/// 		movah, 0
/// 		movbx, ax
/// 		moval, cs:@@end_phase_stage5_map[bx]
/// 		popbx
/// 		jmp@@end_phase_compare
///
/// @@end_phase_reimu_mapdb 2, 3, 5, 7, 9, 10, 11
/// @@end_phase_stage4_mapdb 2, 6, 9, 13, 17
/// @@end_phase_stage5_mapdb 2, 3, 4, 6, 10, 11, 14
///
/// @@end_phase_compare:
/// 		cmp_boss_phase, al
/// 		jbnear ptr @@end_phase_done
/// @@end_phase_reached:
/// 		moves:[bx+resident_t.game_end_flag], GAME_END_SUCCESS
/// 		push30
/// 		callable(@frame_delay$qi
/// 		mov_quit, Q_QUIT_TO_OP
/// ```
/// Returns the number of phases available for the given stage.
/// If more phases are entered. 3 outcomes:
/// 1. The game freezes and you must kill it and restart;
/// 2. The boss alive again;
/// 3. The boss booms and the stage is ended.
///
/// As state and tested:
///
/// Stage 1: 3 Phases
/// Stage 2: 4 Phases
/// Stage 3: 6 Phases
/// Stage 4: For Marisa it is 4 and for Reimu it is 6.
/// Stage 5: 5 Phases.
/// Stage 6: 7 Phases
/// Extra Stage: 1-2 is first boss and 3-8 are second boss.
/// E.g., max_phase(0) returns 4, meaning phases 0-4 are valid (5 phases), so you might need to +1
fn max_phase(stage: u8, char: u8) -> u8 {
    // Just panic here because it is my error. No one else can make this panic
    // other than a programmer.
    assert!((stage as usize) < 7, "Invalid stage: {}", stage); // ex
    match stage {
        0 => 3,
        1 => 4,
        2 => 6,
        3 => {
            if char <= 1 {
                4
            } else {
                6
            }
        }
        4 => 5,
        5 => 7,
        6 => 8,
        _ => unreachable!(),
    }
}

#[inline]
/// A dumb hard coded function for the maximum selectable value of the stage.
/// Returns the number of phases *selectable* for the given stage.
/// As state and tested:
///
/// Stage 1: 3 Phases
/// Stage 2: 4 Phases
/// Stage 3: 6 Phases
/// Stage 4: For Marisa it is 4 and for Reimu it is 6.
/// Stage 5: 5 Phases.
/// Stage 6: 7 Phases
/// Extra Stage: 1-2 is first boss and 3-8 are second boss.
///
/// We minus 1 on all stages to get the difficulty high. Because, if it is low, the agent
/// is very easy to win and very very easy to win.
/// E.g., max_phase(0) returns 4, meaning phases 0-4 are valid (5 phases), so you might need to +1
fn selectable_phase(stage: u8, char: u8) -> u8 {
    assert!((stage as usize) < 7, "Invalid stage: {}", stage); // ex
    match stage {
        0 => 2,
        1 => 3,
        2 => 5,
        3 => {
            if char <= 1 {
                3
            } else {
                5
            }
        }
        4 => 4,
        5 => 6,
        6 => 7,
        _ => unreachable!(),
    }
}

pub fn cfg_read(path: &Path) -> io::Result<Cfg04a> {
    let content = std::fs::read_to_string(path)?;

    let mut cfg = Cfg04a::default();

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

pub fn cfg_write(path: &Path, cfg: &Cfg04a) -> io::Result<()> {
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
pub fn specific_cfg_gen(stage: u8, _phase: u8, rank: u8, cha: u8) -> Cfg04a {
    let mut cfg = Cfg04a::default();
    cfg.rank = rank;
    cfg.cha = cha;
    if stage == 6 {
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
