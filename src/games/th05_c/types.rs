use serde::{Deserialize, Serialize};

/*
    TH05 types and structures of rrr.
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

pub type Subpixel = i16; //

#[repr(C)]
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PlayfieldMotion {
    pub cur_x: Subpixel,
    pub cur_y: Subpixel,
    pub prev_x: Subpixel,
    pub prev_y: Subpixel,
    pub vel_x: Subpixel,
    pub vel_y: Subpixel,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Laser {
    pub flag: u8,
    pub col: u8,
    pub origin_x: Subpixel,
    pub origin_y: Subpixel,
    pub starts_at_distance: Subpixel,
    pub ends_at_distance: Subpixel,
    pub angle: u8,
    pub width: u8,
    pub shootout_speed: Subpixel,
    pub age: i32,
    pub active_at_age: i32,
    pub shrink_at_age: i16,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Bullet {
    pub flag: u8,
    pub age: u8,
    pub pos: PlayfieldMotion,
    pub from_group: u8,
    pub speed_cur: u8,
    pub angle: u8,
    pub spawn_flag: u8,
    pub move_flag: u8,
    pub special_motion: u8,
    pub speed_final: u8,
    pub decel_time_or_turns: u8,
    pub decel_delta_or_angle: u8,
    pub patnum: i16,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Enemy {
    pub flag: u8,
    pub age: u8,
    pub pos: PlayfieldMotion,
    pub hp: i16,
    pub score: i16,
    pub script: u16,
    pub script_ip: i16,
    pub speed: u8,
    pub patnum_base: u8,
    pub cur_instr_frame: u8,
    pub loop_i: u8,
    pub angle: u8,
    pub angle_delta: u8,
    pub anim_cels: u8,
    pub anim_frames_per_cel: u8,
    pub anim_cur_cel: u8,
    pub clip: i8,
    pub item: u8,
    pub damaged_this_frame: u8,
    pub can_be_damaged: u8,
    pub autofire: u8,
    pub kills_player_on_collision: u8,
    pub spawned_in_left_half: u8,
    pub autofire_cur_frame: u8,
    pub autofire_interval: u8,
    pub bullet_template: [u8; 20],
    pub subtype: u8,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Item {
    pub flag: u8,
    pub pos: PlayfieldMotion,
    pub item_type: u8,
    pub patnum: i16,
    pub pulled_to_player: u16,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CustomEntity {
    pub flag: u8,
    pub angle: u8,
    pub pos: PlayfieldMotion,
    pub val1: u16,
    pub val2: u16,
    pub sprite: i16,
    pub val3: i16,
    pub damage: i16,
    pub speed: u8,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CheetoTrail {
    pub flag: u8, // CF_FREE=0, CF_DECELERATE=1, CF_SPEEDUP=2
    pub col: i8,
    pub node_pos: [PlayfieldPoint; 16], // 16 nodes, each is 2*i16 = 4 bytes
    pub node_sprite: [u8; 16],          // 16 sprite indices
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PlayfieldPoint {
    pub x: Subpixel,
    pub y: Subpixel,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Firewave {
    pub alive: u8,
    pub is_right: u8,
    pub bottom: i16,
    pub amp: i16,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Boss {
    pub pos: PlayfieldMotion,
    pub hp: i16,
    pub sprite: u8,
    pub phase: u8,
    pub phase_frame: i16,
    pub damage_this_frame: u8,
    pub mode: u8,
    pub angle: u8,
    pub patterns_seen: u8,
    pub phase_end_hp: i16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResidentState {
    pub rem_lives: u8,
    pub credit_lives: u8,
    pub rem_bombs: u8,
    pub credit_bombs: u8,
    pub stage: u8,
    pub rank: u8,
    pub playchar: u8,
    pub stage_ascii: u8,
    pub rand: u32,
    pub bgm_mode: u8,
    pub se_mode: u8,
    pub shottype: u8,
    pub debug_mode: u8,
    pub end_type_ascii: u8,
    pub end_sequence: u8,
    pub score: u64,
    pub graze: u16,
    pub miss_count: u8,
    pub bombs_used: u8,
    pub items_spawned: u16,
    pub items_collected: u16,
    pub point_items_collected: u16,
    pub max_valued_point_items_collected: u16,
    pub enemies_killed: u16,
    pub enemies_gone: u16,
    pub frames: u32,
    pub slow_frames: u32,
    pub std_frames: u16,
    pub demo_stage: u8,
    pub demo_num: u8,
    pub zunsoft_shown: u8,
    pub turbo_mode: u8,
    pub game_end_flag: u8,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StageCollectionState {
    pub point_items_stage: u8,
    pub dream_items: u8,
    pub stage_graze: u16,
    pub dream_score: u16,
}

impl PlayfieldMotion {
    pub fn to_pixels(&self) -> (f32, f32) {
        (self.cur_x as f32 / 16.0, self.cur_y as f32 / 16.0)
    }

    pub fn velocity_pixels(&self) -> (f32, f32) {
        (self.vel_x as f32 / 16.0, self.vel_y as f32 / 16.0)
    }
}

impl Bullet {
    pub fn get_pixel_pos(&self) -> (f32, f32) {
        self.pos.to_pixels()
    }

    pub fn is_active(&self) -> bool {
        self.flag != 0
    }
}

impl Enemy {
    pub fn get_pixel_pos(&self) -> (f32, f32) {
        self.pos.to_pixels()
    }

    pub fn is_active(&self) -> bool {
        self.flag != 0 && self.hp > 0
    }
}

impl Item {
    pub fn get_pixel_pos(&self) -> (f32, f32) {
        self.pos.to_pixels()
    }

    pub fn is_active(&self) -> bool {
        self.flag != 0
    }
}

impl CustomEntity {
    pub fn is_active(&self) -> bool {
        self.flag != 0
    }
}

impl Boss {
    pub fn get_pixel_pos(&self) -> (f32, f32) {
        self.pos.to_pixels()
    }

    pub fn is_active(&self) -> bool {
        self.hp > 0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerState {
    pub pos: PlayfieldMotion,
    pub power: u8,
    pub invincibility_time: u16,
    pub invincible_via_bomb: bool,
    pub miss_frame: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameState {
    pub resident: ResidentState,
    pub player: PlayerState,
    pub bullets: Vec<Bullet>,
    pub enemies: Vec<Enemy>,
    pub items: Vec<Item>,
    pub boss: Option<Boss>,
    pub boss_2: Option<Boss>,
    pub midboss: Option<Boss>,
    pub lasers: Vec<Laser>,
    pub cheeto_trails: Vec<CheetoTrail>,
    pub custom_entities: Vec<CustomEntity>,
    pub firewaves: Vec<Firewave>,
    pub stage_collection: StageCollectionState,
    /// This was a bug by me until the morning I see that the agent
    /// learnt never press bomb I found it is wrong.
    /// Must written in `update_bomb_tracking`
    pub rem_bombs_internal: u8,
}

impl GameState {
    pub fn get_active_bullets(&self) -> Vec<&Bullet> {
        self.bullets.iter().filter(|b| b.is_active()).collect()
    }

    pub fn get_active_enemies(&self) -> Vec<&Enemy> {
        self.enemies.iter().filter(|e| e.is_active()).collect()
    }

    pub fn get_active_items(&self) -> Vec<&Item> {
        self.items.iter().filter(|i| i.is_active()).collect()
    }

    pub fn update_bomb_tracking(&mut self, prev_state: Option<&GameState>) {
        if let Some(prev) = prev_state {
            if self.resident.miss_count > prev.resident.miss_count {
                self.rem_bombs_internal = self.resident.credit_bombs; // reset
            } else if self.resident.bombs_used > prev.resident.bombs_used {
                self.rem_bombs_internal = prev.rem_bombs_internal.saturating_sub(1);
            } else {
                self.rem_bombs_internal = prev.rem_bombs_internal;
                // avoid trigger the bug and loss the tracking.
            }
        } else {
            self.rem_bombs_internal = self.resident.credit_bombs;
        }
    }
}
