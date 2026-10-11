pub mod cfg;
pub mod key;
pub mod observation;
pub mod offsets;
pub mod readers;
#[path = "../th05c/watcher.rs"]
pub mod watcher;
pub use watcher::TH05CSession as TH04ASession;
/*
    Module (th04A) of thrl.
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
pub struct GameState {
    pub base: super::th05c::GameState,
    pub projectiles: Vec<crate::observation::schema1::Entity>,
    pub player_is_hit: bool,
}

use crate::error::{Error, Result};
use crate::games::common::finder::DiscoveredAddresses;
use crate::memory::{MemoryRegion, ProcessMemory};
use offsets::*;

pub struct DynAddressFinder {
    mem: ProcessMemory,
    addrs: DiscoveredAddresses,
}

impl DynAddressFinder {
    pub fn new(pid: i32) -> Result<Self> {
        Ok(Self {
            mem: ProcessMemory::open(pid)?,
            addrs: DiscoveredAddresses::default(),
        })
    }

    pub fn addresses(&self) -> &DiscoveredAddresses {
        &self.addrs
    }

    pub fn memory(&mut self) -> &mut ProcessMemory {
        &mut self.mem
    }

    pub fn all_structures(&mut self) -> Result<()> {
        let regions: Vec<_> = self
            .mem
            .maps()
            .iter()
            .filter(|region| region.is_readable() && region.is_writable())
            .flat_map(|region| {
                (0..region.size)
                    .step_by(8 * 1024 * 1024)
                    .map(move |offset| MemoryRegion {
                        start: region.start + offset,
                        size: (region.size - offset)
                            .min(8 * 1024 * 1024 + DATA_SIGNATURE.len() - 1),
                        ..region.clone()
                    })
            })
            .collect();
        for anchor in self.mem.search_pattern(DATA_SIGNATURE, Some(&regions)) {
            let data = anchor - DATA_SIGNATURE_OFFSET;
            let entry = data - DATA_SEGMENT * 16;
            if self.mem.read_u8(entry).ok() != Some(0xBA) {
                continue;
            }
            let segment = self.mem.read_u16_le(entry + 1)? as usize;
            let ram = data - segment * 16;
            let pointer = self.mem.read_u32_le(data + RESIDENT)?;
            let resident = ram + (pointer >> 16) as usize * 16 + (pointer & 0xFFFF) as usize;
            if self.mem.try_read(resident, 11).as_deref() != Some(b"HUMAConfig\0") {
                continue;
            }
            self.addrs = DiscoveredAddresses {
                resident: Some(resident),
                player_pos: Some(data + PLAYER),
                ..DiscoveredAddresses::default()
            };
            return Ok(());
        }
        Err(Error::StructureNotFound { name: "th04a" })
    }
}
