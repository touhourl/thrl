//! TH05 Memory reading functions.
//!
//! [rewrite:50]
//!
//! About the ugly unsafe functions, I have something to say if you think I am wrong:
//!
//! 1. Does the game change?
//! 2. Do we know the source?
//! 3. Was the code source in c++?
//! 4. Is using from_le_bytes optimal?
//! 5. Is there bool or unsafe things, UB things where I use unsafe
//!
//! So I just use unsafe here. Th05 has more data to do than th04.

/*
    TH05 memory reader of rrr.
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
// It only runs on little endian machines.
#[cfg(not(target_endian = "little"))]
compile_error!("TH05 assume little-endian layout");

use super::offsets::*;
use super::types::*;
use crate::error::{Error, Result};
use crate::memory::ProcessMemory;

/// Read playfieldmotion struct from a certain addr. Here we use `unsafe` because it is called
/// many times, and it can speed up time a little bit, therefore it can leave more time for my
/// shitty xpu to calculate.
pub fn playfield_motion(mem: &mut ProcessMemory, addr: usize) -> Result<PlayfieldMotion> {
    let data = mem.read(addr, 12)?;
    Ok(unsafe { *(data.as_ptr() as *const PlayfieldMotion) })
}

/// Read resident state. We here not use `unsafe` becuase here are bools, unions, etc. By I looked
/// at the PINCE (a thing which like cheat engine in linux), the union's structure is quite strange.
/// Therefore, just do so.
pub fn resident_t(
    mem: &mut ProcessMemory,
    addr: usize,
    _player_pos: Option<usize>,
) -> Result<ResidentState> {
    let data = mem.read(addr, 256)?;
    if data.len() < 140 {
        return Err(Error::InvalidGameState {
            reason: "Resident data too small".to_string(),
        });
    }

    //let cfg_power = data[12];
    //let credit_lives = data[13];
    //let credit_bombs = data[14];
    let cfg_lives = data[15];
    let cfg_bombs = data[16];
    let rank = data[17];
    let bgm_mode = data[18];
    let stage = data[19];
    let playchar = data[20];
    let se_mode = data[21];
    let turbo_mode = data[22];
    let debug = data[23];
    let end_sequence = data[26];
    let miss_count = data[27];
    let bombs_used = data[28];
    let demo_stage = data[29];
    let demo_num = data[31];

    let score_last_bytes = &data[32..40];
    let score = decode_bcd_score(score_last_bytes);

    let rand = u32::from_le_bytes([data[40], data[41], data[42], data[43]]);
    let std_frames = u16::from_le_bytes([data[44], data[45]]);
    let items_spawned = u16::from_le_bytes([data[46], data[47]]);
    let items_collected = u16::from_le_bytes([data[48], data[49]]);
    let point_items_collected = u16::from_le_bytes([data[50], data[51]]);
    let max_valued_point_items_collected = u16::from_le_bytes([data[52], data[53]]);
    let enemies_gone = u16::from_le_bytes([data[54], data[55]]);
    let enemies_killed = u16::from_le_bytes([data[56], data[57]]);
    let graze = u16::from_le_bytes([data[58], data[59]]);
    let slow_frames = u32::from_le_bytes([data[60], data[61], data[62], data[63]]);
    let frames = u32::from_le_bytes([data[64], data[65], data[66], data[67]]);

    let game_end_flag = data[126];

    let zunsoft_shown = data[9]; // unused... and it is always 0 since we change op.exe.

    Ok(ResidentState {
        rem_lives: cfg_lives.saturating_sub(miss_count),
        credit_lives: cfg_lives,
        rem_bombs: 0,
        credit_bombs: cfg_bombs,
        stage,
        rank,
        playchar,
        stage_ascii: stage,
        rand,
        bgm_mode,
        se_mode,
        shottype: playchar,
        debug_mode: debug,
        end_type_ascii: 0,
        end_sequence,
        score,
        graze,
        miss_count,
        bombs_used,
        items_spawned,
        items_collected,
        point_items_collected,
        max_valued_point_items_collected,
        enemies_killed,
        enemies_gone,
        frames,
        slow_frames,
        std_frames,
        demo_stage,
        demo_num,
        zunsoft_shown,
        turbo_mode,
        game_end_flag,
    })
}

pub fn bullet(mem: &mut ProcessMemory, addr: usize) -> Result<Bullet> {
    let data = mem.read(addr, TH05CStride::BULLET_STRIDE)?;

    if data.len() < 26 {
        return Err(Error::InvalidGameState {
            reason: "Bullet data too small".to_string(),
        });
    }

    let pos = unsafe { *(data[2..14].as_ptr() as *const PlayfieldMotion) };

    Ok(Bullet {
        flag: data[0],
        age: data[1],
        pos,
        from_group: data[14],
        speed_cur: data[16],
        angle: data[17],
        spawn_flag: data[18],
        move_flag: data[19],
        special_motion: data[20],
        speed_final: data[21],
        decel_time_or_turns: data[22],
        decel_delta_or_angle: data[23],
        patnum: i16::from_le_bytes([data[24], data[25]]),
    })
}

/// Read all bullets from array.
pub fn all_bullets(mem: &mut ProcessMemory, bullets_addr: usize) -> Vec<Bullet> {
    let mut bullets = Vec::with_capacity(TH05ArrayLength::BULLET_COUNT);

    for i in 0..TH05ArrayLength::BULLET_COUNT {
        let addr = bullets_addr + i * TH05CStride::BULLET_STRIDE;
        if let Ok(bullet) = bullet(mem, addr)
            && bullet.flag != 0
        {
            bullets.push(bullet);
        }
    }

    bullets
}

/// Read a single item from address.
pub fn item(mem: &mut ProcessMemory, addr: usize) -> Option<Item> {
    let data = mem.try_read(addr, TH05CStride::ITEM_STRIDE)?;

    if data.len() < TH05CStride::ITEM_STRIDE {
        return None;
    }

    let flag = data[0];
    if !matches!(flag, 0..=2) {
        return None;
    }

    let pos = unsafe { *(data[2..14].as_ptr() as *const PlayfieldMotion) };

    let item_type = data[14];

    if flag != 0 && !matches!(item_type, 0..=6) {
        return None;
    }

    let (x, y) = pos.to_pixels();
    if !(-50.0..=TH05Config::PLAYFIELD_W + 50.0).contains(&x)
        || !(-50.0..=TH05Config::PLAYFIELD_H + 50.0).contains(&y)
    {
        return None;
    }

    let patnum = i16::from_le_bytes([data[16], data[17]]);
    let pulled_to_player = u16::from_le_bytes([data[18], data[19]]);

    Some(Item {
        flag,
        pos,
        item_type,
        patnum,
        pulled_to_player,
    })
}

pub fn all_items(mem: &mut ProcessMemory, items_addr: usize) -> Vec<Item> {
    let total_size = TH05ArrayLength::ITEM_COUNT * TH05CStride::ITEM_STRIDE;
    let Ok(data) = mem.read(items_addr, total_size) else {
        return Vec::new();
    };

    (0..TH05ArrayLength::ITEM_COUNT)
        .filter_map(|i| {
            let offset = i * TH05CStride::ITEM_STRIDE;
            let d = data.get(offset..offset + TH05CStride::ITEM_STRIDE)?;
            let flag = d[0];
            if flag == 0 || !matches!(flag, 1 | 2) {
                return None;
            }
            let item_type = d[14];
            if !matches!(item_type, 0..=6) {
                return None;
            }
            let pos = unsafe { *(d[2..14].as_ptr() as *const PlayfieldMotion) };
            let (x, y) = pos.to_pixels();
            if !(-50.0..=TH05Config::PLAYFIELD_W + 50.0).contains(&x)
                || !(-50.0..=TH05Config::PLAYFIELD_H + 50.0).contains(&y)
            {
                return None;
            }
            Some(Item {
                flag,
                pos,
                item_type,
                patnum: i16::from_le_bytes([d[16], d[17]]),
                pulled_to_player: u16::from_le_bytes([d[18], d[19]]),
            })
        })
        .collect()
}

/// Read a single enemy from address.
pub fn read_enemy(mem: &mut ProcessMemory, addr: usize) -> Option<Enemy> {
    let data = mem.try_read(addr, TH05CStride::ENEMY_STRIDE)?;

    if data.len() < 64 {
        return None;
    }

    let flag = data[0];
    if !matches!(flag, 1 | 3) {
        return None;
    }

    let hp = i16::from_le_bytes([data[14], data[15]]);
    if hp <= 0 || hp >= 500 {
        return None;
    }

    let score = i16::from_le_bytes([data[16], data[17]]);
    if score < 0 {
        return None;
    }

    let pos = unsafe { *(data[2..14].as_ptr() as *const PlayfieldMotion) };
    let (x, y) = pos.to_pixels();
    if !(1.0..=TH05Config::PLAYFIELD_W + 100.0).contains(&x)
        || !(-100.0..=TH05Config::PLAYFIELD_H + 100.0).contains(&y)
    {
        return None;
    }

    Some(Enemy {
        flag,
        age: data[1],
        pos,
        hp,
        score,
        script: u16::from_le_bytes([data[18], data[19]]),
        script_ip: i16::from_le_bytes([data[20], data[21]]),
        speed: data[22],
        patnum_base: data[23],
        cur_instr_frame: data[24],
        loop_i: data[25],
        angle: data[26],
        angle_delta: data[27],
        anim_cels: data[28],
        anim_frames_per_cel: data[29],
        anim_cur_cel: data[30],
        clip: data[31] as i8,
        item: data[32],
        damaged_this_frame: data[33],
        can_be_damaged: data[34],
        autofire: data[35],
        kills_player_on_collision: data[36],
        spawned_in_left_half: data[37],
        autofire_cur_frame: data[38],
        autofire_interval: data[39],
        bullet_template: [0; 20],
        subtype: data[60],
    })
}

pub fn all_enemies(mem: &mut ProcessMemory, enemies_addr: usize) -> Vec<Enemy> {
    let mut enemies = Vec::with_capacity(TH05ArrayLength::ENEMY_COUNT);

    for i in 0..TH05ArrayLength::ENEMY_COUNT {
        if let Some(enemy) = read_enemy(mem, enemies_addr + i * TH05CStride::ENEMY_STRIDE)
            && enemy.flag != 0
        {
            enemies.push(enemy);
        }
    }

    enemies
}

pub fn boss_at(mem: &mut ProcessMemory, addr: Option<usize>) -> Option<Boss> {
    let addr = addr?;
    let data = mem.try_read(addr, 24)?;

    let pos = unsafe { *(data[0..12].as_ptr() as *const PlayfieldMotion) };

    Some(Boss {
        pos,
        hp: i16::from_le_bytes([data[12], data[13]]),
        sprite: data[14],
        phase: data[15],
        phase_frame: i16::from_le_bytes([data[16], data[17]]),
        damage_this_frame: data[18],
        mode: data[19],
        angle: data[20],
        patterns_seen: data[21],
        phase_end_hp: i16::from_le_bytes([data[22], data[23]]),
    })
}

pub fn midboss(
    mem: &mut ProcessMemory,
    pos_addr: Option<usize>,
    hp_addr: Option<usize>,
) -> Option<Boss> {
    let pos_addr = pos_addr?;
    let hp_addr = hp_addr?;

    // HP is i16, not i32!
    let hp = mem.read_i16_le(hp_addr).ok()?;
    if hp <= 0 || hp > 10000 {
        return None;
    }

    let pos_data = mem.try_read(pos_addr, 12)?;
    let pos = unsafe { *(pos_data.as_ptr() as *const PlayfieldMotion) };

    let (x, y) = pos.to_pixels();
    // Midboss position must be within playfield bounds.
    if !(0.0..=TH05Config::PLAYFIELD_W).contains(&x)
        || !(0.0..=TH05Config::PLAYFIELD_H).contains(&y)
    {
        return None;
    }

    Some(Boss {
        pos,
        hp,
        sprite: 0,
        phase: 0,
        phase_frame: 0,
        damage_this_frame: 0,
        mode: 0,
        angle: 0,
        patterns_seen: 0,
        phase_end_hp: 0,
    })
}

pub fn normalize_boss_entity(
    mem: &mut ProcessMemory,
    entity: Option<Boss>,
    pos_addr: Option<usize>,
    hp_raw: Option<i16>,
) -> Option<Boss> {
    let hp = hp_raw.filter(|&hp| (1..=30000).contains(&hp))?;

    if let Some(mut e) = entity {
        e.hp = hp;
        return Some(e);
    }

    let pos = playfield_motion(mem, pos_addr?).ok()?;

    Some(Boss {
        pos,
        hp,
        sprite: 0,
        phase: 0,
        phase_frame: 0,
        damage_this_frame: 0,
        mode: 0,
        angle: 0,
        patterns_seen: 0,
        phase_end_hp: 0,
    })
}

pub fn stage_collection_state(
    mem: &mut ProcessMemory,
    stage_point_items_addr: Option<usize>,
    dream_items_addr: Option<usize>,
    stage_graze_addr: Option<usize>,
    dream_score_addr: Option<usize>,
) -> StageCollectionState {
    let mut state = StageCollectionState::default();

    if let Some(addr) = stage_point_items_addr
        && let Ok(v) = mem.read_u8(addr)
    {
        state.point_items_stage = v;
    }

    if let Some(addr) = dream_items_addr
        && let Ok(v) = mem.read_u8(addr)
    {
        state.dream_items = v;
    }

    if let Some(addr) = stage_graze_addr
        && let Ok(v) = mem.read_u16_le(addr)
    {
        state.stage_graze = v;
    }

    if let Some(addr) = dream_score_addr
        && let Ok(v) = mem.read_u16_le(addr)
    {
        state.dream_score = v * 10;
    }

    state
}

fn decode_bcd_score(digits: &[u8]) -> u64 {
    let mut score = 0u64;
    for &digit in digits.iter().take(8) {
        if digit > 9 {
            return 0;
        }
        score = score * 10 + digit as u64;
    }
    score
}

/// Currently unused. Also
///
/// [copy]
#[allow(unused)]
fn live_score(mem: &mut ProcessMemory, player_pos: usize) -> Option<u64> {
    let score_addr = player_pos.checked_sub(0x526)?;
    let score_raw = mem.try_read(score_addr, 8)?;

    if score_raw.len() != 8 || !score_raw.iter().all(|&b| b <= 9) {
        return None;
    }

    Some(decode_bcd_score(&score_raw))
}

pub fn custom_entity(mem: &mut ProcessMemory, addr: usize) -> Option<CustomEntity> {
    let data = mem.try_read(addr, TH05CStride::CE_STRIDE)?;

    if data.len() < TH05CStride::CE_STRIDE {
        return None;
    }

    let flag = data[0];
    if flag == 0 {
        return None;
    }

    let sprite = i16::from_le_bytes([data[18], data[19]]);

    let pos = unsafe { *(data[2..14].as_ptr() as *const PlayfieldMotion) };

    let (x, y) = pos.to_pixels();
    if !(-200.0..=TH05Config::PLAYFIELD_W + 200.0).contains(&x)
        || !(-200.0..=TH05Config::PLAYFIELD_H + 200.0).contains(&y)
    {
        return None;
    }

    if sprite == 0 {
        return None;
    }

    Some(CustomEntity {
        flag,
        angle: data[1],
        pos,
        val1: u16::from_le_bytes([data[14], data[15]]),
        val2: u16::from_le_bytes([data[16], data[17]]),
        sprite,
        val3: i16::from_le_bytes([data[20], data[21]]),
        damage: i16::from_le_bytes([data[22], data[23]]),
        speed: data[24],
    })
}

pub fn all_custom_entities(mem: &mut ProcessMemory, custom_addr: usize) -> Vec<CustomEntity> {
    let mut entities = Vec::with_capacity(TH05ArrayLength::CUSTOM_COUNT);

    for i in 0..TH05ArrayLength::CUSTOM_COUNT {
        if let Some(entity) = custom_entity(mem, custom_addr + i * TH05CStride::CE_STRIDE)
            && entity.flag != 0
        {
            entities.push(entity);
        }
    }

    entities
}

pub fn firewave(mem: &mut ProcessMemory, addr: usize) -> Option<Firewave> {
    let data = mem.try_read(addr, TH05CStride::FIREWAVE_STRIDE)?;

    if data.len() < TH05CStride::FIREWAVE_STRIDE {
        return None;
    }

    Some(Firewave {
        alive: data[0],
        is_right: data[1],
        bottom: i16::from_le_bytes([data[2], data[3]]),
        amp: i16::from_le_bytes([data[4], data[5]]),
    })
}

pub fn all_firewaves(mem: &mut ProcessMemory, firewave_addr: usize) -> Vec<Firewave> {
    let mut firewaves = Vec::with_capacity(TH05ArrayLength::FIREWAVE_COUNT);

    for i in 0..TH05ArrayLength::FIREWAVE_COUNT {
        if let Some(firewave) = firewave(mem, firewave_addr + i * TH05CStride::FIREWAVE_STRIDE)
            && firewave.alive != 0
        {
            firewaves.push(firewave);
        }
    }

    firewaves
}

pub fn laser(mem: &mut ProcessMemory, addr: usize) -> Option<Laser> {
    let data = mem.try_read(addr, TH05CStride::LASER_STRIDE)?;

    if data.len() < TH05CStride::LASER_STRIDE {
        return None;
    }

    let laser = unsafe { *(data.as_ptr() as *const Laser) };

    // Valid laser flags are 1-7
    if laser.flag == 0 || laser.flag > 7 {
        return None;
    }

    // Validate origin is within good bonds
    let x = laser.origin_x as f32 / 16.0;
    let y = laser.origin_y as f32 / 16.0;
    if !(-200.0..=TH05Config::PLAYFIELD_W + 200.0).contains(&x)
        || !(-200.0..=TH05Config::PLAYFIELD_H + 200.0).contains(&y)
    {
        return None;
    }

    Some(laser)
}

pub fn all_lasers(mem: &mut ProcessMemory, lasers_addr: usize) -> Vec<Laser> {
    let mut lasers = Vec::with_capacity(TH05ArrayLength::LASER_COUNT);

    for i in 0..TH05ArrayLength::LASER_COUNT {
        if let Some(laser) = laser(mem, lasers_addr + i * TH05CStride::LASER_STRIDE) {
            lasers.push(laser);
        }
    }

    lasers
}

pub fn cheeto_trail(mem: &mut ProcessMemory, addr: usize) -> Option<CheetoTrail> {
    let data = mem.try_read(addr, TH05CStride::CHEETO_STRIDE)?;

    if data.len() < TH05CStride::CHEETO_STRIDE {
        return None;
    }

    let trail = unsafe { *(data.as_ptr() as *const CheetoTrail) };

    // Cheeto trail flag: CF_FREE=0, CF_DECELERATE=1, CF_SPEEDUP=2
    if trail.flag == 0 || trail.flag > 2 {
        return None;
    }

    // Validate first node position
    let x = trail.node_pos[0].x as f32 / 16.0;
    let y = trail.node_pos[0].y as f32 / 16.0;
    if !(-200.0..=TH05Config::PLAYFIELD_W + 200.0).contains(&x)
        || !(-200.0..=TH05Config::PLAYFIELD_H + 200.0).contains(&y)
    {
        return None;
    }

    Some(trail)
}

pub fn all_cheeto_trails(mem: &mut ProcessMemory, cheeto_addr: usize) -> Vec<CheetoTrail> {
    let mut trails = Vec::with_capacity(TH05ArrayLength::CHEETO_TRAIL_COUNT);

    for i in 0..TH05ArrayLength::CHEETO_TRAIL_COUNT {
        if let Some(trail) = cheeto_trail(mem, cheeto_addr + i * TH05CStride::CHEETO_STRIDE) {
            trails.push(trail);
        }
    }

    trails
}
