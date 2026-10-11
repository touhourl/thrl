use super::{DynAddressFinder, GameState, offsets::*};
use crate::error::Result;
// bad usage, we just do what like in rec98 including games
use crate::games::th05c::{readers as common, state::*};
use crate::memory::ProcessMemory;
use crate::observation::schema1::{Entity, Motion};
/*
    Reader (th04a) of thrl.
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
fn word(data: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([data[offset], data[offset + 1]])
}

fn dword(data: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap())
}
// used, but unused.
fn score(digits: &[u8]) -> u64 {
    digits
        .iter()
        .rev()
        .fold(0, |score, &digit| score * 10 + digit as u64)
}

pub fn resident_t(mem: &mut ProcessMemory, addr: usize) -> Result<ResidentState> {
    let d = mem.read(addr, 76)?;
    Ok(ResidentState {
        rem_lives: d[11],
        credit_lives: d[58],
        rem_bombs: d[13],
        credit_bombs: d[59],
        rank: d[15],
        bgm_mode: d[16],
        stage: d[17],
        playchar: (d[18] - b'0') * 2 + d[25], // hack of th04 compared th05
        stage_ascii: d[19],
        rand: dword(&d, 20),
        se_mode: d[24],
        shottype: d[25],
        debug_mode: d[26],
        score: score(&d[29..37]),
        end_type_ascii: d[37],
        std_frames: word(&d, 38),
        items_spawned: word(&d, 40),
        items_collected: word(&d, 42),
        point_items_collected: word(&d, 44),
        max_valued_point_items_collected: word(&d, 46),
        end_sequence: d[48],
        miss_count: d[49],
        bombs_used: d[50],
        enemies_gone: word(&d, 52),
        enemies_killed: word(&d, 54),
        graze: word(&d, 56),
        demo_stage: d[60],
        demo_num: d[62],
        slow_frames: dword(&d, 64),
        frames: dword(&d, 68),
        zunsoft_shown: d[72],
        turbo_mode: d[73],
        game_end_flag: d[74],
    })
}

pub fn read_state(finder: &mut DynAddressFinder) -> Option<GameState> {
    let resident_addr = finder.addresses().resident?;
    let data = finder.addresses().player_pos? - PLAYER;
    let mem = finder.memory();
    let mut resident = resident_t(mem, resident_addr).ok()?;
    resident.score = score(&mem.read(data + SCORE, 8).ok()?);
    let projectiles = projectiles(mem, data, &resident).ok()?;
    let player_is_hit = mem.read_u8(data + PLAYER_IS_HIT).ok()? != 0;
    let base = crate::games::th05c::GameState {
        resident,
        player: PlayerState {
            pos: common::playfield_motion(mem, data + PLAYER).ok()?,
            power: mem.read_u8(data + POWER).ok()?,
            invincibility_time: mem.read_u8(data + INVINCIBILITY).ok()? as u16,
            invincible_via_bomb: mem.read_u8(data + BOMBING).ok()? != 0,
            miss_frame: mem.read_u8(data + MISS_TIME).ok()? as u16,
        },
        bullets: (0..BULLET_COUNT)
            .filter_map(|i| common::bullet(mem, data + BULLETS + i * BULLET_STRIDE).ok())
            .filter(|b| b.flag == 1 && b.spawn_flag <= 2 && b.move_flag < 4)
            .collect(),
        enemies: (0..ENEMY_COUNT)
            .filter_map(|i| enemy(mem, data + ENEMIES + i * ENEMY_STRIDE))
            .collect(),
        items: (0..ITEM_COUNT)
            .filter_map(|i| common::item(mem, data + ITEMS + i * ITEM_STRIDE))
            .filter(Item::is_active)
            .collect(),
        boss: common::boss_at(mem, Some(data + BOSS)).filter(Boss::is_active),
        boss_2: None,
        midboss: if mem.read_u8(data + MIDBOSS_ACTIVE).ok()? != 0 {
            common::midboss(mem, Some(data + MIDBOSS), Some(data + MIDBOSS + 14))
        } else {
            None
        },
        lasers: Vec::new(),
        cheeto_trails: Vec::new(),
        custom_entities: Vec::new(),
        firewaves: Vec::new(),
        stage_collection: common::stage_collection_state(
            mem,
            Some(data + STAGE_POINT),
            Some(data + DREAM_ITEMS),
            Some(data + STAGE_GRAZE),
            Some(data + DREAM_SCORE),
        ),
    };
    Some(GameState {
        base,
        projectiles,
        player_is_hit,
    })
}

/// Read a single enemy from address.
pub fn enemy(mem: &mut ProcessMemory, addr: usize) -> Option<Enemy> {
    let data = mem.try_read(addr, ENEMY_STRIDE)?;

    let flag = data[0];
    if !matches!(flag, 1 | 3) {
        return None;
    }

    let hp = i16::from_le_bytes([data[16], data[17]]);
    let score = i16::from_le_bytes([data[20], data[21]]);
    let pos = unsafe { (data[2..14].as_ptr() as *const PlayfieldMotion).read_unaligned() };
    Some(Enemy {
        flag,
        age: data[1],
        pos,
        hp,
        score,
        script: u16::from_le_bytes([data[22], data[23]]),
        script_ip: i16::from_le_bytes([data[24], data[25]]),
        speed: data[28],
        patnum_base: data[14],
        cur_instr_frame: data[26],
        loop_i: data[27],
        angle: data[30],
        angle_delta: data[31],
        anim_cels: data[37],
        anim_frames_per_cel: data[38],
        anim_cur_cel: data[39],
        clip: data[32] as i8,
        item: data[35],
        damaged_this_frame: data[36],
        can_be_damaged: data[40],
        autofire: data[41],
        kills_player_on_collision: data[42],
        spawned_in_left_half: data[43],
        autofire_cur_frame: data[62],
        autofire_interval: data[63],
        bullet_template: [0; 20],
        subtype: data[14],
    })
}

fn projectiles(
    mem: &mut ProcessMemory,
    data: usize,
    resident: &ResidentState,
) -> Result<Vec<Entity>> {
    let mut entities = Vec::new();
    let (count, type_id) = match (resident.stage, resident.playchar) {
        (3, 0 | 1) => (4, 0.25),
        (3, _) => (32, 0.5),
        (5, _) => (31, 0.75),
        _ => (0, 0.0),
    };
    for i in 0..count {
        let d = mem.read(data + CUSTOM_ENTITIES + i * CUSTOM_STRIDE, CUSTOM_STRIDE)?;
        let active = match (resident.stage, resident.playchar) {
            (3, 0 | 1) => (1..0x80).contains(&d[0]),
            (3, _) => d[0] != 0,
            (5, _) => d[0] == 1,
            _ => false,
        };
        if active {
            entities.push(Entity {
                motion: Motion {
                    x: word(&d, 2) as i16 as f32 / 16.0,
                    y: word(&d, 4) as i16 as f32 / 16.0,
                    vx: if type_id == 0.25 {
                        0.0
                    } else {
                        word(&d, 10) as i16 as f32 / 16.0
                    },
                    vy: if type_id == 0.25 {
                        0.0
                    } else {
                        word(&d, 12) as i16 as f32 / 16.0
                    },
                },
                type_id,
                hitbox: 24.0,
            });
        }
    }
    if resident.stage == 2 && mem.read_u8(data + SCYTHE_ACTIVE)? == 1 {
        let pos = common::playfield_motion(mem, data + SCYTHE_MOTION)?;
        entities.push(Entity {
            motion: Motion {
                x: pos.cur_x as f32 / 16.0,
                y: pos.cur_y as f32 / 16.0,
                vx: pos.vel_x as f32 / 16.0,
                vy: pos.vel_y as f32 / 16.0,
            },
            type_id: 1.0,
            hitbox: 48.0,
        });
    }
    for i in 0..2 {
        let d = mem.read(
            data + THICKLASERS + i * THICKLASER_STRIDE,
            THICKLASER_STRIDE,
        )?;
        if d[0] == 0 {
            continue;
        }
        let radius = if d[0] == 1 { 0.0 } else { word(&d, 20) as f32 };
        let half_width = radius - (radius / 4.0).min(16.0);
        let center_x = word(&d, 2) as i16 as f32 / 16.0;
        let top = word(&d, 4) as i16 as f32 / 16.0 + radius / 2.0;
        let left = (center_x - half_width).max(0.0);
        let right = (center_x + half_width).min(383.0);
        if left > right {
            continue;
        }
        for y in (top.max(0.0).ceil() as usize..368).step_by(4) {
            for i in 0..=((right - left) / 4.0).ceil() as usize {
                let x = (left + i as f32 * 4.0).min(right);
                entities.push(Entity {
                    motion: Motion {
                        x,
                        y: y as f32,
                        vx: 0.0,
                        vy: 0.0,
                    },
                    type_id: if d[0] == 1 { 0.0 } else { 0.125 },
                    hitbox: 0.0,
                });
            }
        }
    }
    Ok(entities)
}
