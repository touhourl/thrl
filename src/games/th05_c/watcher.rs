use crate::games::th05_c::key::{DiscreteAction, key_det_player_pos, shiftkey_player_pos};
use crate::games::th05_c::readers::*;
use crate::games::th05_c::{DynAddressFinder, GameState, PlayerState};
use crate::observation::schema1::ObservationBuilder;

/*
    TH05 memory watcher of rrr.
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
pub struct TH05MemoryWatcher {
    pub finder: DynAddressFinder,
    pub obs_builder: ObservationBuilder,
    pub last_state: Option<GameState>,
    pub current_action: Option<usize>,
    pub keyboard_device: Option<evdev::Device>,
}
/// We do here sh*t code: one over another...
impl TH05MemoryWatcher {
    pub fn new(pid: i32) -> Result<Self, Box<dyn std::error::Error>> {
        let finder = DynAddressFinder::new(pid)?;
        let obs_builder = ObservationBuilder::default();

        let mut keyboard_device = None;
        if let Ok(dir) = std::fs::read_dir("/dev/input") {
            for entry in dir.flatten() {
                let path = entry.path();
                if path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|s| s.starts_with("event"))
                {
                    // check
                    if let Ok(device) = evdev::Device::open(&path) {
                        // Check
                        if let Some(keys) = device.supported_keys()
                            && keys.contains(evdev::KeyCode::KEY_UP)
                                && keys.contains(evdev::KeyCode::KEY_X)
                            {
                                keyboard_device = Some(device);
                                break;
                            }
                    }
                }
            }
        }

        Ok(Self {
            finder,
            obs_builder,
            last_state: None,
            current_action: None,
            keyboard_device,
        })
    }

    pub fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.finder.all_structures()?;
        Ok(())
    }

    pub fn try_read_state(&mut self) -> Option<GameState> {
        // get all addr
        let resident_addr = self.finder.addresses().resident?;
        let player_pos_addr = self.finder.addresses().player_pos?;
        let bullets_addr = self.finder.addresses().bullets;
        let enemies_addr = self.finder.addresses().enemies;
        let items_addr = self.finder.addresses().items;
        let boss_addr = self.finder.addresses().boss;
        let boss_hp_addr = self.finder.addresses().boss_hp;
        let boss_2_addr = self.finder.addresses().boss_2;
        let boss_2_hp_addr = self.finder.addresses().boss_2_hp;
        let midboss_addr = self.finder.addresses().midboss;
        let midboss_hp_addr = self.finder.addresses().midboss_hp;
        let stage_graze_addr = self.finder.addresses().stage_graze;

        let mem = self.finder.memory();

        let resident = resident_t(mem, resident_addr, Some(player_pos_addr)).ok()?;

        let player_pos = playfield_motion(mem, player_pos_addr).ok()?;
        let power = mem.read_u8(player_pos_addr + 0x1E).ok()?;
        let invincibility_time = mem.read_u16_le(player_pos_addr + 0x1C).ok()?;

        let player = PlayerState {
            pos: player_pos,
            power,
            invincibility_time,
            invincible_via_bomb: false,
            miss_frame: 0,
        };
        // read all
        let bullets = bullets_addr
            .map(|addr| all_bullets(mem, addr))
            .unwrap_or_default();
        let enemies = enemies_addr
            .map(|addr| all_enemies(mem, addr))
            .unwrap_or_default();
        let items = items_addr
            .map(|addr| all_items(mem, addr))
            .unwrap_or_default();

        let boss_entity = boss_at(mem, boss_addr);
        let boss_hp = boss_hp_addr.and_then(|addr| mem.read_i16_le(addr).ok());
        let boss = normalize_boss_entity(mem, boss_entity, boss_addr, boss_hp);

        let boss_2_entity = boss_at(mem, boss_2_addr);
        let boss_2_hp = boss_2_hp_addr.and_then(|addr| mem.read_i16_le(addr).ok());
        let boss_2 = normalize_boss_entity(mem, boss_2_entity, boss_2_addr, boss_2_hp);

        let midboss = midboss(mem, midboss_addr, midboss_hp_addr);

        let lasers_addr = (player_pos_addr as isize - 0x6E98) as usize;
        let lasers = all_lasers(mem, lasers_addr);

        let cheeto_addr = (player_pos_addr as isize - 0x4FE) as usize;
        let cheeto_trails = all_cheeto_trails(mem, cheeto_addr);

        let custom_addr = (player_pos_addr as isize - 0x1230) as usize;
        let custom_entities = all_custom_entities(mem, custom_addr);

        let firewave_addr = (player_pos_addr as isize - 0x48) as usize;
        let firewaves = all_firewaves(mem, firewave_addr);

        let stage_collection = stage_collection_state(
            mem,
            Some(player_pos_addr + 0x26),
            Some(player_pos_addr + 0x23),
            stage_graze_addr,
            None,
        );

        let mut state = GameState {
            resident,
            player,
            bullets,
            enemies,
            items,
            boss,
            boss_2,
            midboss,
            lasers,
            cheeto_trails,
            custom_entities,
            firewaves,
            stage_collection,
            rem_bombs_internal: 0,
        };

        state.update_bomb_tracking(self.last_state.as_ref());
        self.last_state = Some(state.clone());

        Some(state)
    }

    pub fn observation_vec(&self, state: &GameState) -> Vec<f32> {
        self.obs_builder.build_flattened(state)
    }

    pub fn observation_comp(&self, state: &GameState) -> (Vec<f32>, Vec<f32>) {
        self.obs_builder.build_components(state)
    }

    pub fn apply_action(&mut self, action: usize) -> Result<(), Box<dyn std::error::Error>> {
        let discrete_action = DiscreteAction::from_index(action).ok_or("Invalid action index")?;
        let player_pos = self
            .finder
            .addresses()
            .player_pos
            .ok_or("Player pos not found")?;
        let key_det_addr = key_det_player_pos(player_pos);
        let shiftkey_addr = shiftkey_player_pos(player_pos);

        discrete_action.apply(self.finder.memory(), key_det_addr, shiftkey_addr)?; // call
        self.current_action = Some(action);
        Ok(())
    }

    pub fn release_action(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.current_action.is_some() {
            let player_pos = self
                .finder
                .addresses()
                .player_pos
                .ok_or("Player pos not found")?;
            let key_det_addr = key_det_player_pos(player_pos);
            let shiftkey_addr = shiftkey_player_pos(player_pos);

            // Write zero to release all keys
            self.finder.memory().write_u16_le(key_det_addr, 0)?;
            self.finder.memory().write_u8(shiftkey_addr, 0)?;
            self.current_action = None;
        }
        Ok(())
    }
}
