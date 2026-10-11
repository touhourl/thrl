use super::GameState;
use crate::games::th05c;
use crate::observation::{Frame, Schema, schema1 as s1};
/*
    observation (th04A) of thrl.
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
pub fn frame(schema: Schema, mut state: GameState) -> Result<Frame, String> {
    let enemies = std::mem::take(&mut state.base.enemies);
    let bombs = state.base.resident.rem_bombs;
    let mut frame = th05c::observation::frame(schema, state.base)?;
    let Frame::Schema1(ref mut frame1) = frame else {
        unreachable!()
    };
    frame1.rem_bombs_internal = bombs;
    for boss in [&mut frame1.boss, &mut frame1.midboss]
        .into_iter()
        .flatten()
    {
        boss.entity.hitbox = 0.0;
    }
    frame1.enemies = enemies
        .into_iter()
        .map(|enemy| s1::Entity {
            motion: s1::Motion {
                x: enemy.pos.cur_x as f32 / 16.0,
                y: enemy.pos.cur_y as f32 / 16.0,
                vx: enemy.pos.vel_x as f32 / 16.0,
                vy: enemy.pos.vel_y as f32 / 16.0,
            },
            type_id: enemy.subtype as f32 / 255.0,
            hitbox: if enemy.kills_player_on_collision != 0 {
                24.0
            } else {
                0.0
            },
        })
        .collect();
    frame1.projectiles = state
        .projectiles
        .iter()
        .map(|entity| s1::Projectile {
            motion: entity.motion,
            type_id: entity.type_id,
            sub_type: 0.0,
        })
        .collect();
    frame1.projectile_map = state.projectiles;
    Ok(frame)
}
