/*
    schema1 types and structures of thrl.
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
#[derive(Clone, Copy, Default)]
pub struct Motion {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
}

#[derive(Clone, Copy, Default)]
pub struct Entity {
    pub motion: Motion,
    pub type_id: f32,
}

#[derive(Clone, Copy, Default)]
pub struct Projectile {
    pub motion: Motion,
    pub type_id: f32,
    pub sub_type: f32,
}

#[derive(Clone, Copy, Default)]
pub struct Boss {
    pub entity: Entity,
    pub hp: i16,
}

#[derive(Clone, Copy, Default)]
pub struct Player {
    pub motion: Motion,
    pub power: u8,
    pub lives: u8,
    pub invincible: bool,
    pub character_norm: f32,
    pub cfg_lives: u8,
    pub cfg_bombs: u8,
}

#[derive(Clone, Copy, Default)]
pub struct State {
    pub stage_norm: f32,
    pub rank_norm: f32,
    pub score: u64,
    pub graze: u16,
    pub misses: u8,
    pub bombs_used: u8,
    pub point_items: u8,
}

#[derive(Default)]
pub struct Frame {
    pub player: Player,
    pub bullets: Vec<Entity>,
    pub enemies: Vec<Entity>,
    pub items: Vec<Entity>,
    pub boss: Option<Boss>,
    pub boss_2: Option<Boss>,
    pub midboss: Option<Boss>,
    pub projectiles: Vec<Projectile>,
    pub projectile_map: Vec<Entity>,
    pub state: State,
    pub end: u8,
    /// This was a bug by me until the morning I see that the agent
    /// learnt never press bomb I found it is wrong.
    /// Must written in `update_bomb_tracking`
    pub rem_bombs_internal: u8,
}

impl Frame {
    pub fn update_bomb_tracking(&mut self, prev_state: Option<&Frame>) {
        if let Some(prev) = prev_state {
            if self.state.misses > prev.state.misses {
                self.rem_bombs_internal = self.player.cfg_bombs; // reset
            } else if self.state.bombs_used > prev.state.bombs_used {
                self.rem_bombs_internal = prev.rem_bombs_internal.saturating_sub(1);
            } else {
                self.rem_bombs_internal = prev.rem_bombs_internal;
                // avoid trigger the bug and loss the tracking.
            }
        } else {
            self.rem_bombs_internal = self.player.cfg_bombs;
        }
    }

    pub fn history(&self) -> Self {
        Self {
            player: self.player,
            boss: self.boss,
            boss_2: self.boss_2,
            midboss: self.midboss,
            state: self.state,
            end: self.end,
            rem_bombs_internal: self.rem_bombs_internal,
            ..Self::default()
        }
    }
}
