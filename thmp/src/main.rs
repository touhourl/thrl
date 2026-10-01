use eframe::egui::{self, Color32, CornerRadius, Pos2, Rect, Sense, Vec2};
use thrl::{
    games::th05c::{observation as th05_observation, watcher::TH05CSession},
    observation::{
        schema1::{Observation, ObservationBuilder, SpatialMap},
        Frame, Schema,
    },
    param::RuntimeConfig,
};
use std::collections::HashMap;
use std::sync::LazyLock;
use std::time::{Duration, Instant};

/*
    Map viewer of thrl (thmp).
    Observation viewer of RL-rs.
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
struct SchemaUi {
    maps: &'static [&'static str],
    channels: &'static [&'static str],
}

static SCHEMAS: LazyLock<HashMap<&'static str, SchemaUi>> = LazyLock::new(|| {
    HashMap::from([(
        "schema1",
        SchemaUi {
            maps: &["Bullet", "Enemy", "Projectile", "Boss", "Combined"],
            channels: &[
                "Occupancy",
                "Velocity X",
                "Velocity Y",
                "Closest distance",
                "Entity type",
                "Player distance",
            ],
        },
    )])
});

struct App {
    session: TH05CSession,
    builder: ObservationBuilder,
    observation: Option<Observation>,
    schema: &'static SchemaUi,
    map: usize,
    channel: usize,
    paused: bool,
    last_read: Instant,
}

impl App {
    fn new(session: TH05CSession, schema: &'static SchemaUi) -> Self {
        Self {
            session,
            builder: ObservationBuilder::default(),
            observation: None,
            schema,
            map: 0,
            channel: 0,
            paused: false,
            last_read: Instant::now() - Duration::from_secs(1),
        }
    }
    #[inline]
    fn previous_map(&mut self) {
        let len = self.schema.maps.len();
        self.map = (self.map + len - 1) % len;
    }
    #[inline]
    fn next_map(&mut self) {
        let len = self.schema.maps.len();
        self.map = (self.map + 1) % len;
    }
    #[inline]
    fn previous_channel(&mut self) {
        let len = self.schema.channels.len();
        self.channel = (self.channel + len - 1) % len;
    }
    #[inline]
    fn next_channel(&mut self) {
        let len = self.schema.channels.len();
        self.channel = (self.channel + 1) % len;
    }
}
/// Buggy thing got removed; "pause"
/// This shit will get our players lose control to the game.
/// That's the negative side of using -19 and -18.
impl eframe::App for App {
    fn logic(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        // self.keys(ctx); pause it in game.

        if !self.paused && self.last_read.elapsed() >= Duration::from_millis(36) {
            if let Some(state) = self.session.read_state() {
                match th05_observation::frame(Schema::Schema1, state) {
                    Ok(Frame::Schema1(frame)) => {
                        self.observation = Some(self.builder.build_observation(&frame));
                    }
                    Err(e) => {
                        eprintln!("Failed to convert TH05 state to observation frame: {e}");
                    }
                }
            }
            self.last_read = Instant::now();
        }

        ctx.request_repaint_after(Duration::from_millis(16));
    }

    fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
        egui::Panel::top("controls").show(ui, |ui| {
            ui.horizontal(|ui| {
                if ui.button("<").clicked() {
                    self.previous_map();
                }
                ui.strong(self.schema.maps[self.map]);
                if ui.button(">").clicked() {
                    self.next_map();
                }
                ui.separator();
                if ui.button("<").clicked() {
                    self.previous_channel();
                }
                ui.strong(self.schema.channels[self.channel]);
                if ui.button(">").clicked() {
                    self.next_channel();
                }
            });
        });

        egui::CentralPanel::default().show(ui, |ui| {
            let Some(observation) = &self.observation else {
                return;
            };

            let maps = [
                &observation.bullet_map,
                &observation.enemy_map,
                &observation.projectile_map,
                &observation.boss_map,
            ];
            let combined = self.map == 4;
            let map = if combined { maps[0] } else { maps[self.map] };
            let values = channel(map, self.channel);
            let width = ui.available_width();
            let size = Vec2::new(width, width * map.span_y_px / map.span_x_px);
            let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
            let painter = ui.painter_at(rect);
            let cw = rect.width() / map.grid_w as f32;
            let ch = rect.height() / map.grid_h as f32;
            for y in 0..map.grid_h {
                for x in 0..map.grid_w {
                    let i = y * map.grid_w + x;
                    let c = if !combined {
                        Color32::from_gray((level(values[i], self.channel) * 255.0) as u8)
                    } else {
                        let colors = [
                            [255.0_f32, 80.0, 80.0],
                            [80.0, 255.0, 80.0],
                            [80.0, 180.0, 255.0],
                            [255.0, 80.0, 255.0],
                        ];
                        let mut rgb = [0.0_f32; 3];
                        for (m, color) in maps.iter().zip(colors) {
                            let occ = m.occupancy[i].clamp(0.0, 1.0);
                            let v = occ
                                * if self.channel == 0 {
                                    1.0
                                } else {
                                    0.25 + 0.75 * level(channel(m, self.channel)[i], self.channel)
                                };
                            for n in 0..3 {
                                rgb[n] += color[n] * v;
                            }
                        }
                        Color32::from_rgb(
                            rgb[0].min(255.0) as u8,
                            rgb[1].min(255.0) as u8,
                            rgb[2].min(255.0) as u8,
                        )
                    };
                    let min = Pos2::new(rect.left() + x as f32 * cw, rect.top() + y as f32 * ch);
                    painter.rect_filled(
                        Rect::from_min_size(min, Vec2::new(cw + 0.5, ch + 0.5)),
                        CornerRadius::ZERO,
                        c,
                    );
                }
            }

            if let Some(pos) = response.hover_pos() {
                let x = (((pos.x - rect.left()) / rect.width()) * map.grid_w as f32) as usize;
                let y = (((pos.y - rect.top()) / rect.height()) * map.grid_h as f32) as usize;
                if x < map.grid_w && y < map.grid_h {
                    let i = y * map.grid_w + x;
                    if !combined {
                        ui.monospace(format!("cell ({x}, {y}) = {:.6}", values[i]));
                    } else {
                        ui.monospace(format!(
                            "cell ({x}, {y})  B:{:.3} E:{:.3} P:{:.3} Boss:{:.3}",
                            channel(maps[0], self.channel)[i],
                            channel(maps[1], self.channel)[i],
                            channel(maps[2], self.channel)[i],
                            channel(maps[3], self.channel)[i],
                        ));
                    }
                }
            }
        });
    }
}

fn channel(m: &SpatialMap, i: usize) -> &[f32] {
    [
        &m.occupancy,
        &m.velocity_x,
        &m.velocity_y,
        &m.closest_dist,
        &m.entity_type,
        &m.player_dist,
    ][i]
}

fn level(v: f32, channel: usize) -> f32 {
    (if matches!(channel, 1 | 2) {
        (v + 1.0) * 0.5
    } else {
        v
    })
    .clamp(0.0, 1.0)
}

fn main() -> Result<(), String> {
    let cfg = RuntimeConfig::load()?;

    if cfg.runtime.game != "th05c" {
        return Err(format!("thmp does not support game {:?}", cfg.runtime.game));
    }

    let schema_name = format!("schema{}", cfg.runtime.schema);
    let schema = SCHEMAS.get(schema_name.as_str()).ok_or_else(|| {
        format!(
            "We do not have UI metadata for schema {} yet",
            cfg.runtime.schema
        )
    })?;
    if cfg.runtime.schema != 1 {
        return Err(format!(
            "We only supports schema 1, not schema {} yet",
            cfg.runtime.schema
        ));
    }
    rfd::MessageDialog::new()
        .set_title("THMP")
        .set_description("Remember to adjust the window size! ")
        .set_buttons(rfd::MessageButtons::Ok)
        .show();

    let session = TH05CSession::spawn(&cfg)?; // spawn later

    eframe::run_native(
        "thmp",
        eframe::NativeOptions::default(),
        Box::new(move |_| Ok(Box::new(App::new(session, schema)))),
    )
    .map_err(|e| e.to_string())
}
