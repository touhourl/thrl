//! [copy]
use super::{
    BossFeatures,
    BossMap,
    BulletFeature,
    BulletMap,
    DropFeatures,
    EnemyMap,
    PlayerFeatures,
    ProjectileFeature,
    ProjectileMap,
    StateFeatures,
    extract_bullet_entities,
    // DropMap replaced by DropFeatures
    // LaserMap merged into ProjectileMap
    // FirewaveMap merged
    // CheetoMap
    // CustomEntityMap
    extract_projectile_entities,
};
use crate::observation::frame::Frame;
use crate::param::ObservationConfig;
use serde::{Deserialize, Serialize};

/*
    Observation builder of RL-rs.
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservationBuilder {
    pub grid_w: usize,
    pub grid_h: usize,
    pub span_x_px: f32,
    pub span_y_px: f32,
}

impl Default for ObservationBuilder {
    fn default() -> Self {
        Self {
            grid_w: ObservationConfig::GRID_W,
            grid_h: ObservationConfig::GRID_H,
            span_x_px: ObservationConfig::SPAN_X_PX,
            span_y_px: ObservationConfig::SPAN_Y_PX,
        }
    }
}

impl ObservationBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_grid_size(mut self, grid_w: usize, grid_h: usize) -> Self {
        self.grid_w = grid_w;
        self.grid_h = grid_h;
        self
    }

    pub fn with_span(mut self, span_x_px: f32, span_y_px: f32) -> Self {
        self.span_x_px = span_x_px;
        self.span_y_px = span_y_px;
        self
    }

    pub fn build_observation(&self, state: &Frame) -> Observation {
        let player = PlayerFeatures::from_game_state(state);
        let boss = BossFeatures::from_game_state(state);
        let game_state = StateFeatures::from_game_state(state);

        let bullet_map = BulletMap::from_game_state(
            state,
            self.grid_w,
            self.grid_h,
            self.span_x_px,
            self.span_y_px,
        );

        let enemy_map = EnemyMap::from_game_state_enemies(
            state,
            self.grid_w,
            self.grid_h,
            self.span_x_px,
            self.span_y_px,
        );

        let (px, py) = state.player.pos.to_pixels();

        let projectile_map = ProjectileMap::from_all_projectiles(
            &state.lasers,
            &state.firewaves,
            &state.cheeto_trails,
            &state.custom_entities,
            px,
            py,
            self.grid_w,
            self.grid_h,
            self.span_x_px,
            self.span_y_px,
        );

        let boss_map = BossMap::from_game_state_bosses(
            state,
            self.grid_w,
            self.grid_h,
            self.span_x_px,
            self.span_y_px,
        );

        // Entity-level features: top-16 nearest projectiles as direct MLP input
        let projectile_entities = extract_projectile_entities(
            &state.lasers,
            &state.firewaves,
            &state.cheeto_trails,
            &state.custom_entities,
            px,
            py,
            self.span_x_px,
            self.span_y_px,
        );

        // Top-16 nearest bullets as direct MLP input
        let bullet_entities = extract_bullet_entities(state, self.span_x_px, self.span_y_px);

        // Drop items as direct features (nearest 4)
        let drop_features = DropFeatures::from_game_state(state, self.span_x_px, self.span_y_px);

        Observation {
            player,
            boss,
            game_state,
            bullet_map,
            enemy_map,
            projectile_map,
            boss_map,
            projectile_entities,
            bullet_entities,
            drop_features,
        }
    }

    pub fn build_flattened(&self, state: &Frame) -> Vec<f32> {
        let obs = self.build_observation(state);
        obs.to_flattened()
    }

    pub fn build_components(&self, state: &Frame) -> (Vec<f32>, Vec<f32>) {
        let obs = self.build_observation(state);
        (obs.to_feature_vec(), obs.to_map_tensor())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Observation {
    pub player: PlayerFeatures,
    pub boss: BossFeatures,
    pub game_state: StateFeatures,
    pub bullet_map: BulletMap,
    pub enemy_map: EnemyMap,
    pub projectile_map: ProjectileMap,
    pub boss_map: BossMap,
    #[serde(skip)]
    pub projectile_entities: Vec<f32>,
    #[serde(skip)]
    pub bullet_entities: Vec<f32>,
    #[serde(skip)]
    pub drop_features: DropFeatures,
}

impl Observation {
    pub fn to_flattened(&self) -> Vec<f32> {
        let mut vec = Vec::new();

        vec.extend(self.to_feature_vec());
        vec.extend(self.to_map_tensor());

        vec
    }

    /// All scalar/entity features: 273
    pub fn to_feature_vec(&self) -> Vec<f32> {
        let mut vec = Vec::new();
        vec.extend(self.player.to_vec());
        vec.extend(self.boss.to_vec());
        vec.extend(self.game_state.to_vec());
        vec.extend(&self.projectile_entities);
        vec.extend(&self.bullet_entities);
        vec.extend(&self.drop_features.features);
        vec
    }

    /// Spatial maps: bullet + enemy + projectile(merged) + boss = 4 maps * 6 channels = 24 channels
    pub fn to_map_tensor(&self) -> Vec<f32> {
        let mut vec = Vec::new();
        vec.extend(self.bullet_map.to_flattened());
        vec.extend(self.enemy_map.to_flattened());
        vec.extend(self.projectile_map.to_flattened());
        vec.extend(self.boss_map.to_flattened());
        vec
    }

    pub fn feature_count(grid_w: usize, grid_h: usize) -> usize {
        let map_size = grid_w * grid_h;

        Self::feature_only_count() + map_size * 6 * Self::MAP_COUNT
    }

    // 273?
    pub fn feature_only_count() -> usize {
        PlayerFeatures::feature_count()
            + BossFeatures::feature_count()
            + StateFeatures::feature_count()
            + ProjectileFeature::TOTAL_FEATURES
            + BulletFeature::TOTAL_FEATURES
            + DropFeatures::TOTAL_FEATURES
    }

    /// 4 maps * 6 channels = 24
    pub const fn map_channel_count() -> usize {
        Self::MAP_COUNT * 6
    }

    /// Number of spatial maps: bullet, enemy, projectile(merged), boss
    const MAP_COUNT: usize = 4;
}
