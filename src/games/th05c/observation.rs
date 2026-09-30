use super::state as th05;
use crate::observation::{Schema, frame::Frame as ObservationFrame, schema1 as s1};

#[inline]
fn motion(pos: th05::PlayfieldMotion) -> s1::Motion {
    s1::Motion {
        x: pos.cur_x as f32 / 16.0,
        y: pos.cur_y as f32 / 16.0,
        vx: pos.vel_x as f32 / 16.0,
        vy: pos.vel_y as f32 / 16.0,
    }
}

#[inline]
fn entity(pos: th05::PlayfieldMotion, type_id: f32) -> s1::Entity {
    s1::Entity {
        motion: motion(pos),
        type_id,
    }
}

#[inline]
fn boss(value: th05::Boss, type_id: f32) -> s1::Boss {
    s1::Boss {
        entity: entity(value.pos, type_id),
        hp: value.hp,
    }
}

/// My original algorithm was false. I wrongly used table instead of index due to my shitty
/// asm skills. Also, where it is not on it, will not be here.
fn sample_firewave(
    firewave: &th05::Firewave,
    span_x_px: f32,
    span_y_px: f32,
) -> Vec<(f32, f32, bool)> {
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
                span_x_px - x_offset
            } else {
                x_offset + 16.0
            };

            if x >= 0.0 && x <= span_x_px {
                if is_right {
                    // this will be the right side
                    let start_x = x.ceil() as i32;
                    let end_x = span_x_px.floor() as i32;

                    for fill_x in start_x..=end_x {
                        points.push((fill_x as f32, y, is_right));
                    }
                } else {
                    // This goes to the left
                    let start_x = 16;
                    let end_x = x.floor() as i32;

                    for fill_x in start_x..=end_x {
                        points.push((fill_x as f32, y, is_right));
                    }
                }
            }
        }
        y -= 1.0;
        angle += 0.5;
    }

    points
}

/// 0.0=laser, 0.33=firewave, 0.66=cheeto, 1.0=custom
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

/// Firewave map, ExAlice Phase 2 (you see) or 4 (code).
/// Create a firewave map by sampling points along the sine wave.
/// TODO: fill? Or don't fill?
/// Cheeto trail map

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
/// Custom entity map. I only see 05 use it.
/// Create a custom entity map from custom entities.
fn projectiles(
    lasers: &[th05::Laser],
    firewaves: &[th05::Firewave],
    cheeto_trails: &[th05::CheetoTrail],
    custom_entities: &[th05::CustomEntity],
    span_x_px: f32,
    span_y_px: f32,
) -> (Vec<s1::Projectile>, Vec<s1::Entity>) {
    let mut projectiles = Vec::new();
    let mut projectile_map = Vec::new();

    // Lasers: sample points along each beam (same as LaserMap but we keep individual points)
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
        // Sample fewer points for entity list (every 32px instead of 16)
        let num_samples = ((beam_length / 32.0).ceil() as i32 + 1).min(8);
        for i in 0..num_samples {
            let t = if num_samples > 1 {
                i as f32 / (num_samples - 1) as f32
            } else {
                0.5
            };
            let dist_along = start_dist + t * (end_dist - start_dist);
            projectiles.push(s1::Projectile {
                motion: s1::Motion {
                    x: origin_x + cos_a * dist_along,
                    y: origin_y + sin_a * dist_along,
                    vx: 0.0,
                    vy: 0.0,
                },
                type_id: 0.0,
                sub_type: laser.flag as f32 / 7.0,
            });
        }

        let num_samples = beam_length.ceil() as i32 + 1;
        for i in 0..num_samples {
            let t = if num_samples > 1 {
                i as f32 / (num_samples - 1) as f32
            } else {
                0.5
            };
            let dist_along = start_dist + t * (end_dist - start_dist);
            projectile_map.push(s1::Entity {
                motion: s1::Motion {
                    x: origin_x + cos_a * dist_along,
                    y: origin_y + sin_a * dist_along,
                    vx: 0.0,
                    vy: 0.0,
                },
                type_id: (laser.flag as f32 / 7.0) * 0.25, // 0.0..0.25 range
            });
        }
    }

    // Firewaves: use shared sampling function
    // Firewaves: category type_id base = 0.25..0.50
    for firewave in firewaves {
        for (x, y, is_right) in sample_firewave(firewave, span_x_px, span_y_px) {
            projectiles.push(s1::Projectile {
                motion: s1::Motion {
                    x,
                    y,
                    vx: 0.0,
                    vy: 0.0,
                },
                type_id: 1.0 / 3.0,
                sub_type: if is_right { 1.0 } else { 0.0 },
            });
            projectile_map.push(s1::Entity {
                motion: s1::Motion {
                    x,
                    y,
                    vx: 0.0,
                    vy: 0.0,
                },
                type_id: 0.25 + if is_right { 0.125 } else { 0.0 }, // 0.25..0.50
            });
        }
    }

    // Cheeto trails sample node
    // Cheeto trails: category type_id base = 0.50..0.75
    for trail in cheeto_trails {
        let mut prev: Option<(f32, f32)> = None;
        for node_i in (1..16).step_by(2).rev() {
            let x = trail.node_pos[node_i].x as f32 / 16.0;
            let y = trail.node_pos[node_i].y as f32 / 16.0;
            projectiles.push(s1::Projectile {
                motion: s1::Motion {
                    x,
                    y,
                    vx: 0.0,
                    vy: 0.0,
                },
                type_id: 2.0 / 3.0,
                sub_type: trail.flag as f32 / 2.0,
            });
            if let Some((prev_x, prev_y)) = prev {
                let dx = x - prev_x;
                let dy = y - prev_y;
                let distance = (dx * dx + dy * dy).sqrt();
                // FIX: There is no gap to excape, agent.
                let steps = distance.ceil().max(1.0) as usize;
                for s in 1..=steps {
                    let t = s as f32 / steps as f32;
                    projectile_map.push(s1::Entity {
                        motion: s1::Motion {
                            x: prev_x + dx * t,
                            y: prev_y + dy * t,
                            vx: 0.0,
                            vy: 0.0,
                        },
                        type_id: 0.50 + (trail.flag as f32 / 2.0) * 0.25,
                    });
                }
            } else {
                // First point
                projectile_map.push(s1::Entity {
                    motion: s1::Motion {
                        x,
                        y,
                        vx: 0.0,
                        vy: 0.0,
                    },
                    type_id: 0.50 + (trail.flag as f32 / 2.0) * 0.25,
                });
            }

            prev = Some((x, y));
        }
    }

    // Custom entities
    // Custom entities: category type_id base = 0.75..1.0
    for custom_entity in custom_entities {
        let motion = motion(custom_entity.pos);
        let sub = ((custom_entity.sprite as i32 + 128) % 256) as f32 / 255.0;
        projectiles.push(s1::Projectile {
            motion,
            type_id: 1.0,
            sub_type: sub,
        });
        projectile_map.push(s1::Entity {
            motion,
            type_id: 0.75 + sub * 0.25,
        });
    }

    (projectiles, projectile_map)
}

fn schema1(state: th05::GameState) -> s1::Frame {
    let th05::GameState {
        resident,
        player,
        bullets,
        enemies,
        items,
        boss: main_boss,
        boss_2,
        midboss,
        lasers,
        cheeto_trails,
        custom_entities,
        firewaves,
        stage_collection,
    } = state;
    let (projectiles, projectile_map) = projectiles(
        &lasers,
        &firewaves,
        &cheeto_trails,
        &custom_entities,
        384.0,
        368.0,
    );

    s1::Frame {
        player: s1::Player {
            motion: motion(player.pos),
            power: player.power,
            lives: resident.rem_lives,
            invincible: player.invincibility_time > 0
                || player.invincible_via_bomb
                || player.miss_frame > 0,
            character_norm: (resident.playchar as f32 / 3.0).clamp(0.0, 1.0),
            cfg_lives: resident.credit_lives,
            cfg_bombs: resident.credit_bombs,
        },
        bullets: bullets
            .into_iter()
            .filter(th05::Bullet::is_active)
            .map(|b| entity(b.pos, (b.patnum.unsigned_abs() % 256) as f32 / 255.0))
            .collect(),
        enemies: enemies
            .into_iter()
            .filter(th05::Enemy::is_active)
            .map(|e| entity(e.pos, e.subtype as f32 / 255.0))
            .collect(),
        items: items
            .into_iter()
            .filter(th05::Item::is_active)
            .map(|i| entity(i.pos, i.item_type as f32 / 6.0))
            .collect(),
        boss: main_boss.map(|b| boss(b, 0.0)),
        boss_2: boss_2.map(|b| boss(b, 1.0 / 3.0)),
        midboss: midboss.map(|b| boss(b, 2.0 / 3.0)),
        projectiles,
        projectile_map,
        state: s1::State {
            stage_norm: (resident.stage as f32 / 6.0).clamp(0.0, 1.0),
            rank_norm: (resident.rank as f32 / 3.0).clamp(0.0, 1.0),
            score: resident.score,
            graze: stage_collection.stage_graze,
            misses: resident.miss_count,
            bombs_used: resident.bombs_used,
            point_items: stage_collection.point_items_stage,
        },
        end: resident.game_end_flag,
        rem_bombs_internal: 0,
    }
}

pub fn frame(schema: Schema, state: th05::GameState) -> Result<ObservationFrame, String> {
    match schema {
        Schema::Schema1 => Ok(ObservationFrame::Schema1(schema1(state))),
    }
}
