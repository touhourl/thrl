pub mod bullets;
pub mod cfg;
pub mod enemies;
pub mod items;
pub mod key;
pub mod offsets;
pub mod player;
pub mod projectiles;
pub mod readers;
pub mod resident;
pub mod types;
pub mod watcher;

pub use types::*;

use crate::error::{Error, Result};
use crate::games::common::finder::DiscoveredAddresses;
use crate::memory::ProcessMemory;
use offsets::*;

/*
    Memory finder of TH05 of rrr.
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

pub struct DynAddressFinder {
    mem: ProcessMemory,
    /// for reader further so we donno seek again
    addrs: DiscoveredAddresses,
}
/// those are just wrappers of functions from other files.
impl DynAddressFinder {
    pub fn new(pid: i32) -> Result<Self> {
        let mem = ProcessMemory::open(pid)?;
        Ok(Self {
            mem,
            addrs: DiscoveredAddresses::default(),
        })
    }

    pub fn addresses(&self) -> &DiscoveredAddresses {
        &self.addrs
    }

    pub fn memory(&mut self) -> &mut ProcessMemory {
        &mut self.mem
    }

    pub fn resident_structure(&mut self) -> Result<usize> {
        let addr = resident::find_resident(&mut self.mem)?;
        self.addrs.resident = Some(addr);
        Ok(addr)
    }

    pub fn player_position_from_resident(&mut self) -> Result<usize> {
        let resident_addr = match self.addrs.resident {
            Some(addr) => addr,
            None => self.resident_structure()?,
        };

        let addr = player::player_position_resident_offset(&mut self.mem, resident_addr)?;
        self.addrs.player_pos = Some(addr);
        self.offset_derived_addresses();
        Ok(addr)
    }

    pub fn player_position(&mut self) -> Result<usize> {
        self.player_position_from_resident()
    }

    pub fn bullet_array(&mut self) -> Result<usize> {
        let player_pos = self.addrs.player_pos.ok_or(Error::StructureNotFound {
            name: "player_pos (for bullets)",
        })?;

        let addr = bullets::bullet_array(&mut self.mem, player_pos)?;
        self.addrs.bullets = Some(addr);
        Ok(addr)
    }

    pub fn enemy_array(&mut self) -> Result<usize> {
        let bullets_addr = match self.addrs.bullets {
            Some(addr) => addr,
            None => self.bullet_array()?,
        };

        let addr = enemies::enemy_array(&mut self.mem, bullets_addr)?;
        self.addrs.enemies = Some(addr);
        Ok(addr)
    }

    pub fn item_array(&mut self) -> Result<usize> {
        let player_pos = self.addrs.player_pos.ok_or(Error::StructureNotFound {
            name: "player_pos (for items)",
        })?;

        let addr = items::item_array(&mut self.mem, player_pos)?;
        self.addrs.items = Some(addr);
        Ok(addr)
    }

    pub fn boss_structures(&mut self) -> Result<()> {
        if self.addrs.player_pos.is_none() {
            self.player_position_from_resident()?;
        }

        self.offset_derived_addresses();

        for (hp_addr, name) in [
            (self.addrs.boss_hp, "boss"),
            (self.addrs.boss_2_hp, "boss_2"),
        ] {
            if let Some(addr) = hp_addr
                && let Ok(hp) = self.mem.read_i16_le(addr)
            {
                tracing::info!("{} hp addr=0x{:08X} value={}", name, addr, hp);
            }
        }

        Ok(())
    }

    pub fn all_structures(&mut self) -> Result<()> {
        self.resident_structure()?;
        self.player_position()?;
        let _ = self.bullet_array(); // no return needed
        let _ = self.boss_structures(); // same
        let _ = self.enemy_array();
        let _ = self.item_array();

        if self.addrs.resident.is_none() || self.addrs.player_pos.is_none() {
            return Err(Error::StructureNotFound {
                name: "critical structures",
            });
        }

        tracing::info!("Successfully found critical structures.");
        Ok(())
    }

    fn offset_derived_addresses(&mut self) {
        let Some(player_pos) = self.addrs.player_pos else {
            return;
        };

        self.addrs.power = Some((player_pos as isize + TH05COffsets::P2POWER as isize) as usize);
        self.addrs.key_det = Some((player_pos as isize + TH05COffsets::P2KEY_DET) as usize);
        self.addrs.boss = Some((player_pos as isize + TH05COffsets::P2BOSS_POS) as usize);
        self.addrs.boss_hp = Some((player_pos as isize + TH05COffsets::P2BOSS_HP) as usize);
        self.addrs.boss_2 = Some((player_pos as isize + TH05COffsets::P2BOSS2_POS) as usize);
        self.addrs.boss_2_hp = Some((player_pos as isize + TH05COffsets::P2BOSS2_HP) as usize);
        self.addrs.midboss = Some((player_pos as isize + TH05COffsets::P2MIDBOSS_POS) as usize);
        self.addrs.midboss_hp = Some((player_pos as isize + TH05COffsets::P2MIDBOSS_HP) as usize);
        self.addrs.bullets = Some((player_pos as isize + TH05COffsets::P2BULLETS) as usize);
        self.addrs.enemies = Some((player_pos as isize + TH05COffsets::P2ENEMIES) as usize);
        self.addrs.items = Some((player_pos as isize + TH05COffsets::P2ITEMS) as usize);
        self.addrs.stage_graze = Some((player_pos as isize + TH05COffsets::P2STAGE_GRAZE) as usize);
    }
}
