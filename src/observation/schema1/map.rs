//! Spatial grids for game entities (bullets, enemies, drops).
//!
//! All three map types are really the same logic applied to different entity types.
//! Instead of duplicating everything three times, I just use a generic SpatialMap that
//! gets specialized for each entity type via a trait. Smartass.
//! I just copied the impl before and ctrl c ctrl v and change a bit.
//!
//! IMPORTANT: Make sure your entity types impl GridEntity correctly
//! the whole grid depends on getting consistent position and velocity data.
//! Some of them are old and not used anymore, I don't wanna see code got deleted so
//! kept for historical reasons.
//!
//! TODO: Remove unused maps after release to github and contributors of thrl project starting to appear
//! See CONTRIBUTING.md:{Writing code}
//!
//! [copy]
//! [paper]

/*
    Map layout of rrr, RL-rs
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
use crate::observation::frame::*;
use serde::{Deserialize, Serialize};

const BULLETMAP_EDGE_SOFTNESS: f32 = 0.2;
const BULLETMAP_EDGE_MIN_WEIGHT: f32 = 0.02;

#[inline]
fn clamp(value: f32, lo: f32, hi: f32) -> f32 {
    value.max(lo).min(hi)
}

pub trait GridEntity {
    fn get_pixel_pos(&self) -> (f32, f32);
    fn get_pixel_velocity(&self) -> (f32, f32);
    fn get_type_id(&self) -> f32 {
        0.0 // unused
    }
}

/// Generic spatial grid with absolute coordinates.
/// it racks occupancy, velocity, closest distance, entity type, and player position for any entity type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpatialMap {
    pub grid_w: usize,
    pub grid_h: usize,
    pub span_x_px: f32,
    pub span_y_px: f32,
    pub active_cells: usize,
    pub active_entities_in_span: usize,
    pub occupancy: Vec<f32>,
    pub velocity_x: Vec<f32>,
    pub velocity_y: Vec<f32>,
    pub closest_dist: Vec<f32>,
    /// Average entity type/ID in each cell (normalized 0-1). Does not from previous project.
    ///
    /// [feature]
    pub entity_type: Vec<f32>,
    pub player_dist: Vec<f32>,
}

impl SpatialMap {
    /// Build a spatial map from any list of entities.
    /// You probably want to use the typed factory methods below instead of this.
    pub fn from_entities<T: GridEntity>(
        entities: impl IntoIterator<Item = T>,
        player_x: f32,
        player_y: f32,
        grid_w: usize,
        grid_h: usize,
        span_x_px: f32,
        span_y_px: f32,
    ) -> Self {
        let size = grid_w * grid_h;
        let mut count = vec![0.0f32; size];
        let mut vx_acc = vec![0.0f32; size];
        let mut vy_acc = vec![0.0f32; size];
        let mut type_acc = vec![0.0f32; size];
        let mut closest = vec![1.0f32; size];

        let max_dist = (span_x_px.powi(2) + span_y_px.powi(2)).sqrt();
        let mut active_in_span = 0usize;

        // Note: we can't get player position here anymore.
        // You'll need to compute (dx, dy) before calling this or pass player pos in.
        // For now, this is a "centered at origin" map.
        for entity in entities {
            let (ex, ey) = entity.get_pixel_pos();
            let (gx, gy, edge_weight, inside) = Self::soft_project_to_grid(
                ex,
                ey,
                grid_w,
                grid_h,
                span_x_px,
                span_y_px,
                BULLETMAP_EDGE_SOFTNESS,
            );
            let gx = gx.min(grid_w - 1);
            let gy = gy.min(grid_h - 1);
            let i = gy * grid_w + gx;

            count[i] += edge_weight;
            let (vx, vy) = entity.get_pixel_velocity();
            vx_acc[i] += vx * edge_weight;
            vy_acc[i] += vy * edge_weight;
            type_acc[i] += entity.get_type_id() * edge_weight;

            let dx = ex - player_x;
            let dy = ey - player_y;
            let dist = (dx * dx + dy * dy).sqrt();
            closest[i] = closest[i].min((dist / max_dist).min(1.0));

            if inside {
                active_in_span += 1;
            }
        }

        let active_cells = count.iter().filter(|&&c| c > 0.0).count();

        let mut occupancy = vec![0.0f32; size];
        let mut velocity_x = vec![0.0f32; size];
        let mut velocity_y = vec![0.0f32; size];
        let mut entity_type = vec![0.0f32; size];
        let mut player_dist = vec![0.0f32; size];

        // Calculate player distance for each cell
        let cell_w = span_x_px / grid_w as f32;
        let cell_h = span_y_px / grid_h as f32;

        for gy in 0..grid_h {
            for gx in 0..grid_w {
                let i = gy * grid_w + gx;

                // Cell center position
                let cell_x = (gx as f32 + 0.5) * cell_w;
                let cell_y = (gy as f32 + 0.5) * cell_h;

                // Distance from cell center to player
                let dx = cell_x - player_x;
                let dy = cell_y - player_y;
                let dist = (dx * dx + dy * dy).sqrt();
                player_dist[i] = (dist / max_dist).clamp(0.0, 1.0);

                if count[i] <= 0.0 {
                    closest[i] = 1.0;
                    continue;
                }
                occupancy[i] = (count[i] / 4.0).min(1.0);
                // We just take the maximum v is 64 px
                velocity_x[i] = clamp((vx_acc[i] / count[i]) / 64.0, -1.0, 1.0);
                velocity_y[i] = clamp((vy_acc[i] / count[i]) / 64.0, -1.0, 1.0);
                entity_type[i] = type_acc[i] / count[i];
            }
        }

        Self {
            grid_w,
            grid_h,
            span_x_px,
            span_y_px,
            active_cells,
            active_entities_in_span: active_in_span,
            occupancy,
            velocity_x,
            velocity_y,
            closest_dist: closest,
            entity_type,
            player_dist,
        }
    }

    #[inline]
    fn soft_project_to_grid(
        x: f32,
        y: f32,
        grid_w: usize,
        grid_h: usize,
        span_x_px: f32,
        span_y_px: f32,
        edge_softness: f32,
    ) -> (usize, usize, f32, bool) {
        // Absolute coordinates: map [0, span] to [0, grid]
        let nx = x / span_x_px;
        let ny = y / span_y_px;

        let inside = (0.0..=1.0).contains(&nx) && (0.0..=1.0).contains(&ny);

        // Edge softness for entities slightly outside playfield
        let weight = if inside {
            1.0
        } else {
            let dx_excess = if nx < 0.0 {
                -nx
            } else if nx > 1.0 {
                nx - 1.0
            } else {
                0.0
            };
            let dy_excess = if ny < 0.0 {
                -ny
            } else if ny > 1.0 {
                ny - 1.0
            } else {
                0.0
            };
            let dx_excess_px = dx_excess * span_x_px;
            let dy_excess_px = dy_excess * span_y_px;
            let excess_px = dx_excess_px.max(dy_excess_px);
            let falloff = 1.0 / (1.0 + (excess_px / edge_softness.max(1e-6)));
            falloff.max(BULLETMAP_EDGE_MIN_WEIGHT)
        };

        let clamped_x = nx.clamp(0.0, 1.0);
        let clamped_y = ny.clamp(0.0, 1.0);
        let gx = (clamped_x * grid_w as f32) as usize;
        let gy = (clamped_y * grid_h as f32) as usize;
        let gx = gx.min(grid_w - 1);
        let gy = gy.min(grid_h - 1);
        (gx, gy, weight, inside)
    }

    /// Flatten all channels into one vector for neural net input.
    pub fn to_flattened(&self) -> Vec<f32> {
        let mut v = Vec::with_capacity(self.occupancy.len() * 6);
        v.extend(&self.occupancy);
        v.extend(&self.velocity_x);
        v.extend(&self.velocity_y);
        v.extend(&self.closest_dist);
        v.extend(&self.entity_type);
        v.extend(&self.player_dist);
        v
    }
}

// Typed wrappers for convenience. These exist so you don't have to impl the
// trait or deal with generics when you just want a bullet map, enemy map, etc.

pub type BulletMap = SpatialMap;

impl BulletMap {
    pub fn from_game_state(
        state: &Frame,
        grid_w: usize,
        grid_h: usize,
        span_x_px: f32,
        span_y_px: f32,
    ) -> Self {
        let (px, py) = state.player.pos.to_pixels();
        let bullets = state.get_active_bullets().into_iter().map(|b| {
            let (bx, by) = b.get_pixel_pos();
            let type_id = (b.patnum.unsigned_abs() % 256) as f32 / 255.0;
            AbsoluteEntity {
                x: bx,
                y: by,
                vx: b.pos.velocity_pixels().0,
                vy: b.pos.velocity_pixels().1,
                type_id,
            }
        });
        Self::from_entities(bullets, px, py, grid_w, grid_h, span_x_px, span_y_px)
    }
}

/// Single bullet entity exposed directly to the MLP.
/// 7 floats: dx, dy, vx, vy, type_id, speed, distance
#[derive(Debug, Clone, Copy)]
pub struct BulletFeature;

impl BulletFeature {
    pub const FEATURE_COUNT: usize = 7;
    pub const MAX_ENTITIES: usize = 16;
    pub const TOTAL_FEATURES: usize = Self::FEATURE_COUNT * Self::MAX_ENTITIES; // 112
}

/// Extract top-K nearest bullets as direct MLP features.
/// Returns K * 7 = 112 floats, zeros if fewer than K bullets active.
pub fn extract_bullet_entities(state: &Frame, span_x_px: f32, span_y_px: f32) -> Vec<f32> {
    let (px, py) = state.player.pos.to_pixels();
    let max_dist = (span_x_px.powi(2) + span_y_px.powi(2)).sqrt();

    let mut bullets_with_dist: Vec<(f32, f32, f32, f32, f32, f32, f32)> = state
        .get_active_bullets()
        .into_iter()
        .map(|b| {
            let (bx, by) = b.get_pixel_pos();
            let dx = bx - px;
            let dy = by - py;
            let (vx, vy) = b.pos.velocity_pixels();
            let dist = (dx * dx + dy * dy).sqrt();
            let speed = (vx * vx + vy * vy).sqrt();
            let type_id = (b.patnum.unsigned_abs() % 256) as f32 / 255.0;
            (dist, dx, dy, vx, vy, type_id, speed)
        })
        .collect();

    // Sort by distance (nearest first)
    bullets_with_dist.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

    let mut result = Vec::with_capacity(BulletFeature::TOTAL_FEATURES);
    for i in 0..BulletFeature::MAX_ENTITIES {
        if i < bullets_with_dist.len() {
            let (dist, dx, dy, vx, vy, type_id, speed) = bullets_with_dist[i];
            result.push((dx / span_x_px).clamp(-1.0, 1.0));
            result.push((dy / span_y_px).clamp(-1.0, 1.0));
            result.push((vx / 12.0).clamp(-1.0, 1.0));
            result.push((vy / 12.0).clamp(-1.0, 1.0));
            result.push(type_id);
            result.push((speed / 12.0).clamp(0.0, 1.0));
            result.push((dist / max_dist).clamp(0.0, 1.0));
        } else {
            result.extend_from_slice(&[0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0]);
        }
    }
    result
}

/// Boss map (tracks main boss, boss_2, and midboss as entities)
pub type BossMap = SpatialMap;

impl BossMap {
    pub fn from_game_state_bosses(
        state: &Frame,
        grid_w: usize,
        grid_h: usize,
        span_x_px: f32,
        span_y_px: f32,
    ) -> Self {
        let (px, py) = state.player.pos.to_pixels();
        let mut bosses = Vec::new();

        // Add main boss if present
        if let Some(boss) = &state.boss
            && boss.hp > 0
        {
            let (bx, by) = boss.pos.to_pixels();
            // Type ID: 0.0 for main boss
            bosses.push(AbsoluteEntity {
                x: bx,
                y: by,
                vx: boss.pos.velocity_pixels().0,
                vy: boss.pos.velocity_pixels().1,
                type_id: 0.0,
            });
        }

        // Add second boss if present
        if let Some(boss_2) = &state.boss_2
            && boss_2.hp > 0
        {
            let (bx, by) = boss_2.pos.to_pixels();
            // Type ID: 0.33 for second boss
            bosses.push(AbsoluteEntity {
                x: bx,
                y: by,
                vx: boss_2.pos.velocity_pixels().0,
                vy: boss_2.pos.velocity_pixels().1,
                type_id: 1.0 / 3.0,
            });
        }

        // Add midboss if present
        if let Some(midboss) = &state.midboss
            && midboss.hp > 0
        {
            let (bx, by) = midboss.pos.to_pixels();
            // Type ID: 0.67 for midboss
            bosses.push(AbsoluteEntity {
                x: bx,
                y: by,
                vx: midboss.pos.velocity_pixels().0,
                vy: midboss.pos.velocity_pixels().1,
                type_id: 2.0 / 3.0,
            });
        }

        Self::from_entities(bosses, px, py, grid_w, grid_h, span_x_px, span_y_px)
    }
}
pub type EnemyMap = SpatialMap;

impl EnemyMap {
    pub fn from_game_state_enemies(
        state: &Frame,
        grid_w: usize,
        grid_h: usize,
        span_x_px: f32,
        span_y_px: f32,
    ) -> Self {
        let (px, py) = state.player.pos.to_pixels();
        let enemies = state.get_active_enemies().into_iter().map(|e| {
            let (ex, ey) = e.get_pixel_pos();
            let type_id = e.subtype as f32 / 255.0;
            AbsoluteEntity {
                x: ex,
                y: ey,
                vx: e.pos.velocity_pixels().0,
                vy: e.pos.velocity_pixels().1,
                type_id,
            }
        });
        Self::from_entities(enemies, px, py, grid_w, grid_h, span_x_px, span_y_px)
    }
}

pub type DropMap = SpatialMap;

impl DropMap {
    pub fn from_game_state_items(
        state: &Frame,
        grid_w: usize,
        grid_h: usize,
        span_x_px: f32,
        span_y_px: f32,
    ) -> Self {
        let (px, py) = state.player.pos.to_pixels();
        let drops = state.get_active_items().into_iter().map(|it| {
            let (ix, iy) = it.get_pixel_pos();
            let type_id = it.item_type as f32 / 6.0;
            AbsoluteEntity {
                x: ix,
                y: iy,
                vx: it.pos.velocity_pixels().0,
                vy: it.pos.velocity_pixels().1,
                type_id,
            }
        });
        Self::from_entities(drops, px, py, grid_w, grid_h, span_x_px, span_y_px)
    }
}

#[derive(Clone)]
struct AbsoluteEntity {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    type_id: f32,
}

impl GridEntity for AbsoluteEntity {
    fn get_pixel_pos(&self) -> (f32, f32) {
        (self.x, self.y)
    }

    fn get_pixel_velocity(&self) -> (f32, f32) {
        (self.vx, self.vy)
    }

    fn get_type_id(&self) -> f32 {
        self.type_id
    }
}

/// Laser map
pub type LaserMap = SpatialMap;

impl LaserMap {
    /// Create a laser map by sampling points along each laser beam.
    ///
    /// Laser Rendering (laser_rh.cpp: laser_render_ray)
    ///
    /// Lasers are rendered as 4-sided trapezoid with width perpendicular to the beam:
    ///
    ///
    /// Calculate perpendicular offset for laser width first:
    ///
    /// Then, calculate 4 corner points of the laser beam;
    ///
    /// Last, clip polygon to screen and render.
    /// grc_clip_polygon_n(&clipped, 8, &corners, 4);
    /// grcg_polygon_cx(&clipped, point_count);
    ///
    ///
    /// Hit detection samples 12*12 boxes every 16 pixels along the centerline.
    ///
    /// Type ID: `flag / 7` (laser types 1-7: shootout, fixed_wait, fixed_grow, fixed_active,
    /// fixed_shrink, fixed_shrink_and_wait, shootout_decay)
    ///
    /// TODO: New type for rendered laser and actually hitbox lasers.
    pub fn from_lasers(
        lasers: &[crate::observation::frame::Laser],
        player_x: f32,
        player_y: f32,
        grid_w: usize,
        grid_h: usize,
        span_x_px: f32,
        span_y_px: f32,
    ) -> Self {
        let mut sampled_points = Vec::new();

        for laser in lasers {
            let origin_x = laser.origin_x as f32 / 16.0;
            let origin_y = laser.origin_y as f32 / 16.0;

            // Convert angle (0-255) to radians
            let angle_rad = (laser.angle as f32 / 256.0) * 2.0 * std::f32::consts::PI;
            let cos_a = angle_rad.cos();
            let sin_a = angle_rad.sin();

            // Sample points along the laser beam centerline
            let start_dist = laser.starts_at_distance as f32 / 16.0;
            let end_dist = laser.ends_at_distance as f32 / 16.0;
            let beam_length = (end_dist - start_dist).abs();

            // Sample every 16 pixels along the beam
            let num_samples = (beam_length / 16.0).ceil() as i32 + 1;
            for i in 0..num_samples {
                let t = if num_samples > 1 {
                    i as f32 / (num_samples - 1) as f32
                } else {
                    0.5
                };
                let dist = start_dist + t * (end_dist - start_dist);

                let x = origin_x + cos_a * dist;
                let y = origin_y + sin_a * dist;

                sampled_points.push(AbsoluteEntity {
                    x,
                    y,
                    vx: 0.0, // Lasers don't move (they grow/shrink but don't translate)
                    vy: 0.0,
                    type_id: laser.flag as f32 / 7.0, // LF_SHOOTOUT=1 to LF_SHOOTOUT_DECAY=7
                });
            }
        }

        Self::from_entities(
            sampled_points,
            player_x,
            player_y,
            grid_w,
            grid_h,
            span_x_px,
            span_y_px,
        )
    }
}

/// My original algorithm was false. I wrongly used table instead of index due to my shitty
/// asm skills. Also, where it is not on it, will not be here.
fn sample_firewave(firewave: &Firewave, span_x_px: f32, span_y_px: f32) -> Vec<(f32, f32, bool)> {
    if firewave.alive == 0 {
        return Vec::new();
    }

    let bottom = firewave.bottom;
    let amp = firewave.amp as f32;
    let is_right = firewave.is_right != 0;

    let mut y = (bottom & !0xF) as f32;
    let mut angle = ((bottom & 0xF) / 2) as f32;
    let mut points = Vec::new();

    while y >= 16.0 && angle < 128.0 {
        // Only keep points inside the visible playfield
        if y <= span_y_px {
            // 8-bit angle, 2^8
            let angle_rad = 2.0 * angle * std::f32::consts::PI / 256.0; // 2 \pi r
            let x_offset = amp * angle_rad.sin();

            let x = if is_right {
                384.0 - x_offset
            } else {
                x_offset + 16.0
            };

            if x >= 0.0 && x <= span_x_px {
                points.push((x, y, is_right));
            }
        }

        y -= 16.0;
        angle += 8.0;
    }

    points
}

/// Firewave map, ExAlice Phase 2 (you see) or 4 (code).
pub type FirewaveMap = SpatialMap;

impl FirewaveMap {
    /// Create a firewave map by sampling points along the sine wave.
    /// TODO: fill? Or don't fill?
    pub fn from_firewaves(
        firewaves: &[crate::observation::frame::Firewave],
        player_x: f32,
        player_y: f32,
        grid_w: usize,
        grid_h: usize,
        span_x_px: f32,
        span_y_px: f32,
    ) -> Self {
        let mut sampled_points = Vec::new();

        for firewave in firewaves {
            for (x, y, is_right) in sample_firewave(firewave, span_x_px, span_y_px) {
                sampled_points.push(AbsoluteEntity {
                    x,
                    y,
                    vx: 0.0,
                    vy: 0.0,
                    type_id: if is_right { 1.0 } else { 0.0 },
                });
            }
        }

        Self::from_entities(
            sampled_points,
            player_x,
            player_y,
            grid_w,
            grid_h,
            span_x_px,
            span_y_px,
        )
    }
}

/// Cheeto trail map
pub type CheetoMap = SpatialMap;

impl CheetoMap {
    /// Create a cheeto trail map by sampling trail nodes.
    ///
    /// From cheeto_u.cpp, cheetos_render.asm
    ///
    /// Cheeto bullets leave a trail of 16 nodes behind them:
    /// ```cpp
    /// ```
    ///
    /// cheetos_render.asm
    /// ```asm
    /// ```
    ///
    /// So it only have 16 nodes and only the idx mod 2 = 1 are rendered.
    /// From like 15, 13, ... 1 (1 is the head, or we can say, 0)
    /// flags: CF_DECELERATE (1) = slowing down, CF_SPEEDUP (2) = speeding up
    pub fn from_cheeto_trails(
        trails: &[crate::observation::frame::CheetoTrail],
        player_x: f32,
        player_y: f32,
        grid_w: usize,
        grid_h: usize,
        span_x_px: f32,
        span_y_px: f32,
    ) -> Self {
        let mut sampled_points = Vec::new();

        for trail in trails {
            // Nodes: See function doc
            for node_i in (1..16).step_by(2).rev() {
                let x = trail.node_pos[node_i].x as f32 / 16.0;
                let y = trail.node_pos[node_i].y as f32 / 16.0;

                sampled_points.push(AbsoluteEntity {
                    x,
                    y,
                    vx: 0.0,
                    vy: 0.0,
                    type_id: trail.flag as f32 / 2.0,
                });
            }
        }

        Self::from_entities(
            sampled_points,
            player_x,
            player_y,
            grid_w,
            grid_h,
            span_x_px,
            span_y_px,
        )
    }
}

/// Custom entity map. I only see 05 use it.
pub type CustomEntityMap = SpatialMap;

impl CustomEntityMap {
    /// Create a custom entity map from custom entities.
    pub fn from_custom_entities(
        entities: &[crate::observation::frame::CustomEntity],
        player_x: f32,
        player_y: f32,
        grid_w: usize,
        grid_h: usize,
        span_x_px: f32,
        span_y_px: f32,
    ) -> Self {
        let mapped = entities.iter().map(|entity| {
            let (x, y) = entity.pos.to_pixels();
            let (vx, vy) = entity.pos.velocity_pixels();
            // Normalize sprite ID to 0-1 range (sprite + 128 to handle negative values, / 255)
            let type_id = ((entity.sprite as i32 + 128) % 256) as f32 / 255.0;
            AbsoluteEntity {
                x,
                y,
                vx,
                vy,
                type_id,
            }
        });
        Self::from_entities(
            mapped, player_x, player_y, grid_w, grid_h, span_x_px, span_y_px,
        )
    }
}

/// Single projectile entity exposed directly to the MLP.
/// 7 floats: dx, dy, vx, vy, type_id, danger, distance
#[derive(Debug, Clone, Copy)]
pub struct ProjectileFeature {
    pub dx: f32,
    pub dy: f32,
    pub vx: f32,
    pub vy: f32,
    pub type_id: f32,
    pub sub_type: f32,
    pub distance: f32,
}

impl ProjectileFeature {
    // Can I really auto detect and delete you???
    pub const FEATURE_COUNT: usize = 7;
    pub const MAX_ENTITIES: usize = 16; // memory limited in ReC98.
    pub const TOTAL_FEATURES: usize = Self::FEATURE_COUNT * Self::MAX_ENTITIES; // 112

    pub fn to_array(&self) -> [f32; Self::FEATURE_COUNT] {
        [
            self.dx,
            self.dy,
            self.vx,
            self.vy,
            self.type_id,
            self.sub_type,
            self.distance,
        ]
    }

    fn zero() -> Self {
        Self {
            dx: 0.0,
            dy: 0.0,
            vx: 0.0,
            vy: 0.0,
            type_id: 0.0,
            sub_type: 0.0,
            distance: 1.0,
        }
    }
}

#[derive(Clone)]
struct RawProjectile {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    /// 0.0=laser, 0.33=firewave, 0.66=cheeto, 1.0=custom
    category: f32,
    sub_type: f32,
}

pub fn extract_projectile_entities(
    lasers: &[crate::observation::frame::Laser],
    firewaves: &[crate::observation::frame::Firewave],
    cheeto_trails: &[crate::observation::frame::CheetoTrail],
    custom_entities: &[crate::observation::frame::CustomEntity],
    player_x: f32,
    player_y: f32,
    span_x_px: f32,
    span_y_px: f32,
) -> Vec<f32> {
    let max_dist = (span_x_px.powi(2) + span_y_px.powi(2)).sqrt();
    let mut all_projectiles: Vec<(f32, RawProjectile)> = Vec::new();

    // Lasers: sample points along each beam (same as LaserMap but we keep individual points)
    for laser in lasers {
        let origin_x = laser.origin_x as f32 / 16.0;
        let origin_y = laser.origin_y as f32 / 16.0;
        let angle_rad = (laser.angle as f32 / 256.0) * 2.0 * std::f32::consts::PI;
        let cos_a = angle_rad.cos();
        let sin_a = angle_rad.sin();
        let start_dist = laser.starts_at_distance as f32 / 16.0;
        let end_dist = laser.ends_at_distance as f32 / 16.0;
        let beam_length = (end_dist - start_dist).abs();
        // Sample fewer points for entity list (every 32px instead of 16)
        let num_samples = ((beam_length / 32.0).ceil() as i32 + 1).min(8);
        for i in 0..num_samples {
            let t = if num_samples > 1 {
                i as f32 / (num_samples - 1) as f32
            } else {
                0.5
            };
            let dist_along = start_dist + t * (end_dist - start_dist);
            let x = origin_x + cos_a * dist_along;
            let y = origin_y + sin_a * dist_along;
            let dx = x - player_x;
            let dy = y - player_y;
            let dist = (dx * dx + dy * dy).sqrt();
            all_projectiles.push((
                dist,
                RawProjectile {
                    x,
                    y,
                    vx: 0.0,
                    vy: 0.0,
                    category: 0.0,
                    sub_type: laser.flag as f32 / 7.0,
                },
            ));
        }
    }

    // Firewaves: use shared sampling function
    for firewave in firewaves {
        for (x, y, is_right) in sample_firewave(firewave, span_x_px, span_y_px) {
            let dx = x - player_x;
            let dy = y - player_y;
            let dist = (dx * dx + dy * dy).sqrt();
            all_projectiles.push((
                dist,
                RawProjectile {
                    x,
                    y,
                    vx: 0.0,
                    vy: 0.0,
                    category: 1.0 / 3.0,
                    sub_type: if is_right { 1.0 } else { 0.0 },
                },
            ));
        }
    }

    // Cheeto trails sample node
    for trail in cheeto_trails {
        for node_i in (1..16).step_by(2).rev() {
            let x = trail.node_pos[node_i].x as f32 / 16.0;
            let y = trail.node_pos[node_i].y as f32 / 16.0;
            let dx = x - player_x;
            let dy = y - player_y;
            let dist = (dx * dx + dy * dy).sqrt();
            all_projectiles.push((
                dist,
                RawProjectile {
                    x,
                    y,
                    vx: 0.0,
                    vy: 0.0,
                    category: 2.0 / 3.0,
                    sub_type: trail.flag as f32 / 2.0,
                },
            ));
        }
    }

    // Custom entities
    for entity in custom_entities {
        let (x, y) = entity.pos.to_pixels();
        let (vx, vy) = entity.pos.velocity_pixels();
        let dx = x - player_x;
        let dy = y - player_y;
        let dist = (dx * dx + dy * dy).sqrt();
        all_projectiles.push((
            dist,
            RawProjectile {
                x,
                y,
                vx,
                vy,
                category: 1.0,
                sub_type: ((entity.sprite as i32 + 128) % 256) as f32 / 255.0,
            },
        ));
    }

    // Sort by distance to player (nearest first)
    all_projectiles.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

    // Take all top-K and convert to features
    let mut result = Vec::with_capacity(ProjectileFeature::TOTAL_FEATURES);
    for i in 0..ProjectileFeature::MAX_ENTITIES {
        let feat = if i < all_projectiles.len() {
            let (dist, ref proj) = all_projectiles[i];
            ProjectileFeature {
                dx: ((proj.x - player_x) / span_x_px).clamp(-1.0, 1.0),
                dy: ((proj.y - player_y) / span_y_px).clamp(-1.0, 1.0),
                vx: (proj.vx / 12.0).clamp(-1.0, 1.0),
                vy: (proj.vy / 12.0).clamp(-1.0, 1.0),
                type_id: proj.category,
                sub_type: proj.sub_type,
                distance: (dist / max_dist).clamp(0.0, 1.0),
            }
        } else {
            ProjectileFeature::zero()
        };
        result.extend_from_slice(&feat.to_array());
    }
    result
}

/// Merged projectile map: combines laser, firewave, cheeto, custom into one Map.
pub type ProjectileMap = SpatialMap;

impl ProjectileMap {
    pub fn from_all_projectiles(
        lasers: &[crate::observation::frame::Laser],
        firewaves: &[crate::observation::frame::Firewave],
        cheeto_trails: &[crate::observation::frame::CheetoTrail],
        custom_entities: &[crate::observation::frame::CustomEntity],
        player_x: f32,
        player_y: f32,
        grid_w: usize,
        grid_h: usize,
        span_x_px: f32,
        span_y_px: f32,
    ) -> Self {
        let mut all_entities: Vec<AbsoluteEntity> = Vec::new();

        // Lasers: category type_id base = 0.0..0.25
        for laser in lasers {
            let origin_x = laser.origin_x as f32 / 16.0;
            let origin_y = laser.origin_y as f32 / 16.0;
            let angle_rad = (laser.angle as f32 / 256.0) * 2.0 * std::f32::consts::PI;
            let cos_a = angle_rad.cos();
            let sin_a = angle_rad.sin();
            let start_dist = laser.starts_at_distance as f32 / 16.0;
            let end_dist = laser.ends_at_distance as f32 / 16.0;
            let beam_length = (end_dist - start_dist).abs();
            let num_samples = (beam_length / 16.0).ceil() as i32 + 1;
            for i in 0..num_samples {
                let t = if num_samples > 1 {
                    i as f32 / (num_samples - 1) as f32
                } else {
                    0.5
                };
                let dist_along = start_dist + t * (end_dist - start_dist);
                let x = origin_x + cos_a * dist_along;
                let y = origin_y + sin_a * dist_along;
                all_entities.push(AbsoluteEntity {
                    x,
                    y,
                    vx: 0.0,
                    vy: 0.0,
                    type_id: (laser.flag as f32 / 7.0) * 0.25, // 0.0..0.25 range
                });
            }
        }

        // Firewaves: category type_id base = 0.25..0.50
        for firewave in firewaves {
            for (x, y, is_right) in sample_firewave(firewave, span_x_px, span_y_px) {
                all_entities.push(AbsoluteEntity {
                    x,
                    y,
                    vx: 0.0,
                    vy: 0.0,
                    type_id: 0.25 + if is_right { 0.125 } else { 0.0 }, // 0.25..0.50
                });
            }
        }

        // Cheeto trails: category type_id base = 0.50..0.75
        for trail in cheeto_trails {
            for node_i in (1..16).step_by(2).rev() {
                let x = trail.node_pos[node_i].x as f32 / 16.0;
                let y = trail.node_pos[node_i].y as f32 / 16.0;
                all_entities.push(AbsoluteEntity {
                    x,
                    y,
                    vx: 0.0,
                    vy: 0.0,
                    type_id: 0.50 + (trail.flag as f32 / 2.0) * 0.25, // 0.50..0.75
                });
            }
        }

        // Custom entities: category type_id base = 0.75..1.0
        for entity in custom_entities {
            let (x, y) = entity.pos.to_pixels();
            let (vx, vy) = entity.pos.velocity_pixels();
            let sub = ((entity.sprite as i32 + 128) % 256) as f32 / 255.0;
            all_entities.push(AbsoluteEntity {
                x,
                y,
                vx,
                vy,
                type_id: 0.75 + sub * 0.25,
            });
        }

        Self::from_entities(
            all_entities,
            player_x,
            player_y,
            grid_w,
            grid_h,
            span_x_px,
            span_y_px,
        )
    }
}

/// Drop item features: extract nearest N items as direct scalar features.
/// Each item (dx, dy, type_id) = 3 floats.
/// Total MAX_ITEMS * 3 = 12 floats.
#[derive(Debug, Clone, Default)]
pub struct DropFeatures {
    pub features: Vec<f32>,
}

impl DropFeatures {
    pub const MAX_ITEMS: usize = 4;
    pub const FEATURES_PER_ITEM: usize = 3;
    pub const TOTAL_FEATURES: usize = Self::MAX_ITEMS * Self::FEATURES_PER_ITEM; // 12

    pub fn from_game_state(
        state: &crate::observation::frame::Frame,
        span_x_px: f32,
        span_y_px: f32,
    ) -> Self {
        let (px, py) = state.player.pos.to_pixels();
        let _max_dist = (span_x_px.powi(2) + span_y_px.powi(2)).sqrt();

        let mut items_with_dist: Vec<(f32, f32, f32, f32)> = state
            .get_active_items()
            .into_iter()
            .map(|it| {
                let (ix, iy) = it.get_pixel_pos();
                let dx = ix - px;
                let dy = iy - py;
                let dist = (dx * dx + dy * dy).sqrt();
                let type_id = it.item_type as f32 / 6.0;
                (dist, dx, dy, type_id)
            })
            .collect();

        items_with_dist.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

        let mut features = Vec::with_capacity(Self::TOTAL_FEATURES);
        for i in 0..Self::MAX_ITEMS {
            if i < items_with_dist.len() {
                let (_, dx, dy, type_id) = items_with_dist[i];
                features.push((dx / span_x_px).clamp(-1.0, 1.0));
                features.push((dy / span_y_px).clamp(-1.0, 1.0));
                features.push(type_id);
            } else {
                features.extend_from_slice(&[0.0, 0.0, 0.0]);
            }
        }
        Self { features }
    }
}
