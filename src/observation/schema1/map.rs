//! Spatial grids for game entities (bullets, enemies, drops).
//!
//! All three map types are really the same logic applied to different entity types.
//! Instead of duplicating everything three times, I just use a generic SpatialMap that
//! gets specialized for each entity type via a trait. Smartass.
//! I just copied the impl before and ctrl c ctrl v and change a bit.
//!
//! IMPORTANT: Make sure your entity types impl GridEntity correctly
//! the whole grid depends on getting consistent position and velocity data.
//! Some of them are old and not used anymore so I delete them.
//!
//! TODO: Remove unused maps after release to github and contributors of thrl project starting to appear
//! See CONTRIBUTING.md:{Writing code}
//!
//! [copy]
//! [paper]

/*
    Map layout of rrr.
    Egocentric Map of RL-rs.
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
use super::frame::*;
use crate::param::RuntimeConfig;
use serde::{Deserialize, Serialize};

const BULLETMAP_EDGE_SOFTNESS: f32 = 0.2;
const BULLETMAP_EDGE_MIN_WEIGHT: f32 = 0.02;

#[inline]
fn clamp(value: f32, lo: f32, hi: f32) -> f32 {
    value.max(lo).min(hi)
}

fn egocentric_map_config() -> Option<(f32, f32)> {
    RuntimeConfig::global().observation.egocentric_map_span()
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
        Self::from_entities_with_mode(
            entities, player_x, player_y, grid_w, grid_h, span_x_px, span_y_px, None,
        )
    }

    fn from_entities_with_mode<T: GridEntity>(
        entities: impl IntoIterator<Item = T>,
        player_x: f32,
        player_y: f32,
        grid_w: usize,
        grid_h: usize,
        span_x_px: f32,
        span_y_px: f32,
        egocentric_map: Option<(f32, f32)>,
    ) -> Self {
        let size = grid_w * grid_h;
        let mut count = vec![0.0f32; size];
        let mut vx_acc = vec![0.0f32; size];
        let mut vy_acc = vec![0.0f32; size];
        let mut type_acc = vec![0.0f32; size];
        let mut closest = vec![1.0f32; size];

        let (projection_span_x_px, projection_span_y_px) =
            egocentric_map.unwrap_or((span_x_px, span_y_px));
        let max_dist = (projection_span_x_px.powi(2) + projection_span_y_px.powi(2)).sqrt();
        let mut active_in_span = 0usize;

        // Note: we can't get player position here anymore.
        // You'll need to compute (dx, dy) before calling this or pass player pos in.
        // For now, this is a "centered at origin" map.

        // side note; the egocentric idea and code is 7 months old!
        for entity in entities {
            let (ex, ey) = entity.get_pixel_pos();
            let dx = ex - player_x;
            let dy = ey - player_y;
            let (gx, gy, edge_weight, inside) = if egocentric_map.is_some() {
                Self::soft_project_to_grid_egocentric(
                    dx,
                    dy,
                    grid_w,
                    grid_h,
                    projection_span_x_px,
                    projection_span_y_px,
                    BULLETMAP_EDGE_SOFTNESS,
                )
            } else {
                Self::soft_project_to_grid(
                    ex,
                    ey,
                    grid_w,
                    grid_h,
                    span_x_px,
                    span_y_px,
                    BULLETMAP_EDGE_SOFTNESS,
                )
            };
            let gx = gx.min(grid_w - 1);
            let gy = gy.min(grid_h - 1);
            let i = gy * grid_w + gx;

            count[i] += edge_weight;
            let (vx, vy) = entity.get_pixel_velocity();
            vx_acc[i] += vx * edge_weight;
            vy_acc[i] += vy * edge_weight;
            type_acc[i] += entity.get_type_id() * edge_weight;

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
        let cell_w = if egocentric_map.is_some() {
            projection_span_x_px * 2.0 / grid_w as f32
        } else {
            span_x_px / grid_w as f32
        };
        let cell_h = if egocentric_map.is_some() {
            projection_span_y_px * 2.0 / grid_h as f32
        } else {
            span_y_px / grid_h as f32
        };

        for gy in 0..grid_h {
            for gx in 0..grid_w {
                let i = gy * grid_w + gx;

                // Cell center position
                let cell_x = (gx as f32 + 0.5) * cell_w;
                let cell_y = (gy as f32 + 0.5) * cell_h;

                // Distance from cell center to player
                let (dx, dy) = if egocentric_map.is_some() {
                    (cell_x - projection_span_x_px, cell_y - projection_span_y_px)
                } else {
                    (cell_x - player_x, cell_y - player_y)
                };
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
    /// This is not experimental feature. It should work.
    /// (this is might not a feature at all)
    #[inline]
    fn soft_project_to_grid_egocentric(
        dx: f32,
        dy: f32,
        grid_w: usize,
        grid_h: usize,
        span_x_px: f32,
        span_y_px: f32,
        edge_softness: f32,
    ) -> (usize, usize, f32, bool) {
        let nx = dx / span_x_px;
        let ny = dy / span_y_px;

        let inside = nx.abs() <= 1.0 && ny.abs() <= 1.0;
        let max_abs = nx.abs().max(ny.abs());
        let weight = if max_abs <= 1.0 {
            1.0
        } else {
            let excess = max_abs - 1.0;
            let falloff = 1.0 / (1.0 + (excess / edge_softness.max(1e-6)));
            falloff.max(BULLETMAP_EDGE_MIN_WEIGHT)
        };

        let clamped_x = clamp(nx, -1.0, 1.0);
        let clamped_y = clamp(ny, -1.0, 1.0);
        let gx = (((clamped_x + 1.0) * 0.5) * grid_w as f32) as usize;
        let gy = (((clamped_y + 1.0) * 0.5) * grid_h as f32) as usize;
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

impl GridEntity for Entity {
    fn get_pixel_pos(&self) -> (f32, f32) {
        (self.motion.x, self.motion.y)
    }

    fn get_pixel_velocity(&self) -> (f32, f32) {
        (self.motion.vx, self.motion.vy)
    }

    fn get_type_id(&self) -> f32 {
        self.type_id
    }
}

pub type BulletMap = SpatialMap;

impl BulletMap {
    pub fn from_game_state(
        state: &Frame,
        grid_w: usize,
        grid_h: usize,
        span_x_px: f32,
        span_y_px: f32,
    ) -> Self {
        Self::from_entities_with_mode(
            state.bullets.iter().copied(),
            state.player.motion.x,
            state.player.motion.y,
            grid_w,
            grid_h,
            span_x_px,
            span_y_px,
            egocentric_map_config(),
        )
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
    let px = state.player.motion.x;
    let py = state.player.motion.y;
    let max_dist = (span_x_px.powi(2) + span_y_px.powi(2)).sqrt();

    let mut bullets_with_dist: Vec<(f32, &Entity)> = state
        .bullets
        .iter()
        .map(|b| {
            let dx = b.motion.x - px;
            let dy = b.motion.y - py;
            ((dx * dx + dy * dy).sqrt(), b)
        })
        .collect();

    // Sort by distance (nearest first)
    bullets_with_dist.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

    let mut result = Vec::with_capacity(BulletFeature::TOTAL_FEATURES);
    for i in 0..BulletFeature::MAX_ENTITIES {
        if let Some((dist, b)) = bullets_with_dist.get(i) {
            let dx = b.motion.x - px;
            let dy = b.motion.y - py;
            let speed = (b.motion.vx * b.motion.vx + b.motion.vy * b.motion.vy).sqrt();
            result.extend_from_slice(&[
                (dx / span_x_px).clamp(-1.0, 1.0),
                (dy / span_y_px).clamp(-1.0, 1.0),
                (b.motion.vx / 12.0).clamp(-1.0, 1.0),
                (b.motion.vy / 12.0).clamp(-1.0, 1.0),
                b.type_id,
                (speed / 12.0).clamp(0.0, 1.0),
                (*dist / max_dist).clamp(0.0, 1.0),
            ]);
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
        let (px, py) = (state.player.motion.x, state.player.motion.y);
        let mut bosses = Vec::new();

        // Add main boss if present
        if let Some(boss) = &state.boss
            && boss.hp > 0
        {
            // Type ID: 0.0 for main boss
            bosses.push(boss.entity);
        }

        // Add second boss if present
        if let Some(boss_2) = &state.boss_2
            && boss_2.hp > 0
        {
            // Type ID: 0.33 for second boss
            bosses.push(boss_2.entity);
        }

        // Add midboss if present
        if let Some(midboss) = &state.midboss
            && midboss.hp > 0
        {
            // Type ID: 0.67 for midboss
            bosses.push(midboss.entity);
        }

        Self::from_entities_with_mode(
            bosses,
            px,
            py,
            grid_w,
            grid_h,
            span_x_px,
            span_y_px,
            egocentric_map_config(),
        )
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
        Self::from_entities_with_mode(
            state.enemies.iter().copied(),
            state.player.motion.x,
            state.player.motion.y,
            grid_w,
            grid_h,
            span_x_px,
            span_y_px,
            egocentric_map_config(),
        )
    }
}

/*
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
*/
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

pub fn extract_projectile_entities(
    projectiles: &[Projectile],
    player_x: f32,
    player_y: f32,
    span_x_px: f32,
    span_y_px: f32,
) -> Vec<f32> {
    let max_dist = (span_x_px.powi(2) + span_y_px.powi(2)).sqrt();
    let mut all_projectiles: Vec<(f32, &Projectile)> = projectiles
        .iter()
        .map(|projectile| {
            let dx = projectile.motion.x - player_x;
            let dy = projectile.motion.y - player_y;
            ((dx * dx + dy * dy).sqrt(), projectile)
        })
        .collect();

    // Sort by distance to player (nearest first)
    all_projectiles.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

    // Take all top-K and convert to features
    let mut result = Vec::with_capacity(ProjectileFeature::TOTAL_FEATURES);
    for i in 0..ProjectileFeature::MAX_ENTITIES {
        let feat = if i < all_projectiles.len() {
            let (dist, projectile) = all_projectiles[i];
            ProjectileFeature {
                dx: ((projectile.motion.x - player_x) / span_x_px).clamp(-1.0, 1.0),
                dy: ((projectile.motion.y - player_y) / span_y_px).clamp(-1.0, 1.0),
                vx: (projectile.motion.vx / 12.0).clamp(-1.0, 1.0),
                vy: (projectile.motion.vy / 12.0).clamp(-1.0, 1.0),
                type_id: projectile.type_id,
                sub_type: projectile.sub_type,
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
        projectiles: &[Entity],
        player_x: f32,
        player_y: f32,
        grid_w: usize,
        grid_h: usize,
        span_x_px: f32,
        span_y_px: f32,
    ) -> Self {
        Self::from_entities_with_mode(
            projectiles.iter().copied(),
            player_x,
            player_y,
            grid_w,
            grid_h,
            span_x_px,
            span_y_px,
            egocentric_map_config(),
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

    pub fn from_game_state(state: &Frame, span_x_px: f32, span_y_px: f32) -> Self {
        let (px, py) = (state.player.motion.x, state.player.motion.y);
        let _max_dist = (span_x_px.powi(2) + span_y_px.powi(2)).sqrt();

        let mut items_with_dist: Vec<(f32, f32, f32, f32)> = state
            .items
            .iter()
            .map(|it| {
                let dx = it.motion.x - px;
                let dy = it.motion.y - py;
                let dist = (dx * dx + dy * dy).sqrt();
                (dist, dx, dy, it.type_id)
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
