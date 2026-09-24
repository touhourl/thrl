use crate::observation::frame::Frame;

/*
    Reward algorithm of rrr.
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
pub struct Reward;
impl Reward {
    const SURVIVAL_REWARD: f32 = 0.1;
    const GRAZE_REWARD: f32 = 1.0;
    const POINT_REWARD: f32 = 1.0;
    const POWER_REWARD: f32 = 1.0;
    const CORNER_REGION_NORMALIZED: f32 = 0.10;
    const CORNER_PENALTY: f32 = -0.5;
    const BOSS_DAMAGE_REWARD: f32 = 0.5;
    pub const DEATH_REWARD: f32 = 50.0;
    const BOMB_REWARD: f32 = 30.0;
    const MIDBOSS_DEFEAT_REWARD: f32 = 50.0;
}

const MIN_X: f32 = 8.0;
const MAX_X: f32 = 376.0;
const MIN_Y: f32 = 8.0;
const MAX_Y: f32 = 352.0;
const RANGE_X: f32 = MAX_X - MIN_X;
const RANGE_Y: f32 = MAX_Y - MIN_Y;

/// For now the positive thing during the bomb period is
/// deprecated!!! Glad we only have 2 things here!!!
///
/// So less complicated. In later games there are enough
/// penalty for the agent.
///
/// `m` for MORL. Typing every time imagine: Multi-
/// Objective Reinforcement Learning Algorithms, We used...
/// Complexity ++, and my finger burn-rs also.
///
/// Returns: $r_\text{survival}, r_\text{fight}, r_\text{resource}$.
pub fn calculate_reward_m(prev_state: Option<&Frame>, curr_state: &Frame) -> Vec<f32> {
    let Some(prev) = prev_state else {
        return vec![0.0, 0.0, 0.0];
    };
    let mut rewards = [0.0f32; 3];

    rewards[0] += Reward::SURVIVAL_REWARD;
    // By this we can address the problem from other papers.
    // Actually it was already done in RL-rs in April. Why did
    // they not thought this penity of corner camping?
    let (px, py) = curr_state.player.pos.to_pixels();
    rewards[0] -= corner_penalty(px, py);

    if curr_state.resident.miss_count > prev.resident.miss_count {
        rewards[0] -= Reward::DEATH_REWARD;
    }

    let graze_delta = positive_u16_delta(
        curr_state.stage_collection.stage_graze,
        prev.stage_collection.stage_graze,
    );
    // Graze belongs to here btw.
    rewards[1] += graze_delta * Reward::GRAZE_REWARD;

    rewards[1] += boss_damage_reward(prev, curr_state);

    if curr_state.resident.miss_count > prev.resident.miss_count {
        let unused_bombs = prev.rem_bombs_internal as f32;
        rewards[2] -= unused_bombs * Reward::BOMB_REWARD;
    }

    if curr_state.resident.bombs_used > prev.resident.bombs_used {
        rewards[2] -= Reward::BOMB_REWARD;
    }

    let power_delta = positive_u8_delta(curr_state.player.power, prev.player.power);
    rewards[2] += power_delta * Reward::POWER_REWARD;

    let point_delta = positive_u8_delta(
        curr_state.stage_collection.point_items_stage,
        prev.stage_collection.point_items_stage,
    );
    rewards[2] += point_delta * Reward::POINT_REWARD;

    rewards.to_vec()
}

pub fn calculate_reward(prev_state: Option<&Frame>, curr_state: &Frame) -> f32 {
    calculate_reward_internal(prev_state, curr_state, true)
}
fn calculate_reward_internal(
    prev_state: Option<&Frame>,
    curr_state: &Frame,
    include_positive_rewards: bool,
) -> f32 {
    let Some(prev) = prev_state else {
        return 0.0;
    };

    let mut reward = 0.0f32;

    if include_positive_rewards {
        reward += Reward::SURVIVAL_REWARD;

        let graze_delta = positive_u16_delta(
            curr_state.stage_collection.stage_graze,
            prev.stage_collection.stage_graze,
        );
        reward += graze_delta * Reward::GRAZE_REWARD;

        reward += boss_damage_reward(prev, curr_state);

        let power_delta = positive_u8_delta(curr_state.player.power, prev.player.power);
        reward += power_delta * Reward::POWER_REWARD;

        let point_delta = positive_u8_delta(
            curr_state.stage_collection.point_items_stage,
            prev.stage_collection.point_items_stage,
        );
        reward += point_delta * Reward::POINT_REWARD;
    }

    if curr_state.resident.miss_count > prev.resident.miss_count {
        reward -= Reward::DEATH_REWARD;
        let unused_bombs = prev.rem_bombs_internal as f32;
        reward -= unused_bombs * Reward::BOMB_REWARD;
    }

    if curr_state.resident.bombs_used > prev.resident.bombs_used {
        reward -= Reward::BOMB_REWARD;
    }

    let (px, py) = curr_state.player.pos.to_pixels();
    reward -= corner_penalty(px, py);

    reward
}

/// Added for 2nd boss in th05.
fn boss_damage_reward(prev_state: &Frame, curr_state: &Frame) -> f32 {
    let mut reward = 0.0f32;

    if let (Some(prev_boss), Some(curr_boss)) = (&prev_state.boss, &curr_state.boss)
        && prev_boss.hp > 0
        && curr_boss.hp > 0
        && curr_boss.hp < prev_boss.hp
    {
        let damage = (prev_boss.hp - curr_boss.hp) as f32;
        reward += damage * Reward::BOSS_DAMAGE_REWARD;
    }

    if let (Some(prev_boss_2), Some(curr_boss_2)) = (&prev_state.boss_2, &curr_state.boss_2)
        && prev_boss_2.hp > 0
        && curr_boss_2.hp > 0
        && curr_boss_2.hp < prev_boss_2.hp
    {
        let damage = (prev_boss_2.hp - curr_boss_2.hp) as f32;
        reward += damage * Reward::BOSS_DAMAGE_REWARD;
    }
    if let (Some(prev_mid), Some(curr_mid)) = (&prev_state.midboss, &curr_state.midboss)
        && prev_mid.hp > 0
        && curr_mid.hp > 0
        && curr_mid.hp < prev_mid.hp
    {
        let damage = (prev_mid.hp - curr_mid.hp) as f32;
        reward += damage * Reward::BOSS_DAMAGE_REWARD;
    }

    let prev_midboss_alive = prev_state.midboss.as_ref().is_some_and(|b| b.hp > 0);
    let curr_midboss_alive = curr_state.midboss.as_ref().is_some_and(|b| b.hp > 0);
    if prev_midboss_alive && !curr_midboss_alive {
        reward += Reward::MIDBOSS_DEFEAT_REWARD;
    }

    reward
}
/// [copy]
///
/// From the old th04 thing, but no still the strict.
fn corner_penalty(px: f32, py: f32) -> f32 {
    let left_dist = ((px - MIN_X) / RANGE_X).clamp(0.0, 1.0);
    let right_dist = ((MAX_X - px) / RANGE_X).clamp(0.0, 1.0);
    let top_dist = ((py - MIN_Y) / RANGE_Y).clamp(0.0, 1.0);
    let bottom_dist = ((MAX_Y - py) / RANGE_Y).clamp(0.0, 1.0);

    let corner_distance = |dx: f32, dy: f32| {
        let nx = dx / Reward::CORNER_REGION_NORMALIZED;
        let ny = dy / Reward::CORNER_REGION_NORMALIZED;

        if nx >= 1.0 || ny >= 1.0 {
            1.0
        } else {
            ((nx * nx + ny * ny).sqrt() / std::f32::consts::SQRT_2).min(1.0)
        }
    };

    let dist_to_corner_normalized = corner_distance(left_dist, top_dist)
        .min(corner_distance(right_dist, top_dist))
        .min(corner_distance(left_dist, bottom_dist))
        .min(corner_distance(right_dist, bottom_dist));

    let corner_factor = (1.0 - dist_to_corner_normalized).powi(2);
    corner_factor * Reward::CORNER_PENALTY.abs()
}

fn positive_u8_delta(curr: u8, prev: u8) -> f32 {
    curr.saturating_sub(prev) as f32
}

fn positive_u16_delta(curr: u16, prev: u16) -> f32 {
    curr.saturating_sub(prev) as f32
}
