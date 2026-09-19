use crate::games::th05_c::types::GameState;
use serde::{Deserialize, Serialize};

/*
    Boss features of rrr, RL-rs.
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
const MAX_DISTANCE_PX: f32 = 500.0;
const VELOCITY_NORM_DIVISOR: f32 = 12.0;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BossFeatures {
    pub present: f32,
    pub hp_norm: f32,
    pub dx_norm: f32,
    pub dy_norm: f32,
    pub vx_norm: f32,
    pub vy_norm: f32,
    pub dist_norm: f32,
    pub boss_2_present: f32,
    pub boss_2_hp_norm: f32,
    pub boss_2_dx_norm: f32,
    pub boss_2_dy_norm: f32,
    pub boss_2_vx_norm: f32,
    pub boss_2_vy_norm: f32,
    pub boss_2_dist_norm: f32,
}

impl BossFeatures {
    pub fn from_game_state(state: &GameState) -> Self {
        let (px, py) = state.player.pos.to_pixels();

        let boss = state
            .boss
            .as_ref()
            .filter(|b| b.is_active())
            .or_else(|| state.midboss.as_ref().filter(|b| b.is_active()));

        let (present, hp_norm, dx_norm, dy_norm, vx_norm, vy_norm, dist_norm) =
            if let Some(boss) = boss {
                let (bx, by) = boss.get_pixel_pos();
                let dx = bx - px;
                let dy = by - py;
                let dist = (dx * dx + dy * dy).sqrt();

                let (vx, vy) = boss.pos.velocity_pixels();

                let hp_norm = (boss.hp.max(0) as f32 / 30000.0).clamp(0.0, 1.0);

                (
                    1.0,
                    hp_norm,
                    (dx / MAX_DISTANCE_PX).clamp(-1.0, 1.0),
                    (dy / MAX_DISTANCE_PX).clamp(-1.0, 1.0),
                    (vx / VELOCITY_NORM_DIVISOR).clamp(-1.0, 1.0),
                    (vy / VELOCITY_NORM_DIVISOR).clamp(-1.0, 1.0),
                    (dist / MAX_DISTANCE_PX).clamp(0.0, 1.0),
                )
            } else {
                (0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0)
            };

        // Boss 2
        let boss_2 = state.boss_2.as_ref().filter(|b| b.is_active());

        let (
            boss_2_present,
            boss_2_hp_norm,
            boss_2_dx_norm,
            boss_2_dy_norm,
            boss_2_vx_norm,
            boss_2_vy_norm,
            boss_2_dist_norm,
        ) = if let Some(boss_2) = boss_2 {
            let (bx, by) = boss_2.get_pixel_pos();
            let dx = bx - px;
            let dy = by - py;
            let dist = (dx * dx + dy * dy).sqrt();

            let (vx, vy) = boss_2.pos.velocity_pixels();

            let hp_norm = (boss_2.hp.max(0) as f32 / 30000.0).clamp(0.0, 1.0);

            (
                1.0,
                hp_norm,
                (dx / MAX_DISTANCE_PX).clamp(-1.0, 1.0),
                (dy / MAX_DISTANCE_PX).clamp(-1.0, 1.0),
                (vx / VELOCITY_NORM_DIVISOR).clamp(-1.0, 1.0),
                (vy / VELOCITY_NORM_DIVISOR).clamp(-1.0, 1.0),
                (dist / MAX_DISTANCE_PX).clamp(0.0, 1.0),
            )
        } else {
            (0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0)
        };

        Self {
            present,
            hp_norm,
            dx_norm,
            dy_norm,
            vx_norm,
            vy_norm,
            dist_norm,
            boss_2_present,
            boss_2_hp_norm,
            boss_2_dx_norm,
            boss_2_dy_norm,
            boss_2_vx_norm,
            boss_2_vy_norm,
            boss_2_dist_norm,
        }
    }

    pub fn to_vec(&self) -> Vec<f32> {
        vec![
            self.present,
            self.hp_norm,
            self.dx_norm,
            self.dy_norm,
            self.vx_norm,
            self.vy_norm,
            self.dist_norm,
            self.boss_2_present,
            self.boss_2_hp_norm,
            self.boss_2_dx_norm,
            self.boss_2_dy_norm,
            self.boss_2_vx_norm,
            self.boss_2_vy_norm,
            self.boss_2_dist_norm,
        ]
    }

    pub const fn feature_count() -> usize {
        14
    }
}
