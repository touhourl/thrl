//! TH05 Player position discovery.

/*
    TH05 Player position discovery of rrr.
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
use super::offsets::*;
use crate::error::{Error, Result};
use crate::memory::ProcessMemory;

/// The old way (spawn value) is easy removed because buggy.
/// Note that fns in rust is not global so we don't care about name coalitions and
/// pragma once.
pub fn player_position_resident_offset(
    mem: &mut ProcessMemory,
    resident_addr: usize,
) -> Result<usize> {
    tracing::info!(
        "Finding player position via resident offset (0x{:X})...",
        TH05COffsets::R2PLAYER
    );

    let player_pos_addr = (resident_addr as isize + TH05COffsets::R2PLAYER) as usize;

    tracing::debug!(
        "Resident: 0x{:08X}, Calculated player_pos: 0x{:08X}",
        resident_addr,
        player_pos_addr
    );

    let data = mem.read(player_pos_addr, 12)?;
    let cur_x = i16::from_le_bytes([data[0], data[1]]);
    let cur_y = i16::from_le_bytes([data[2], data[3]]);
    // we make the bound a bit higher, defensive but should never been negative value...
    if !(-32 * 16..=416 * 16).contains(&cur_x) || !(-32 * 16..=480 * 16).contains(&cur_y) {
        return Err(Error::ValidationFailed {
            reason: format!(
                "Position out of bounds: ({:.1}, {:.1})",
                cur_x as f32 / 16.0,
                cur_y as f32 / 16.0
            ),
        });
    }
    // I did not see that asm the vel and there is non 0 possibility
    // that the player is dying
    let vel_x = i16::from_le_bytes([data[8], data[9]]);
    let vel_y = i16::from_le_bytes([data[10], data[11]]);

    tracing::info!(
        "Found player_pos at 0x{:08X} - Position: ({:.1}, {:.1}), Velocity: ({:.2}, {:.2})",
        player_pos_addr,
        cur_x as f32 / 16.0,
        cur_y as f32 / 16.0,
        vel_x as f32 / 16.0,
        vel_y as f32 / 16.0
    );

    Ok(player_pos_addr)
}
