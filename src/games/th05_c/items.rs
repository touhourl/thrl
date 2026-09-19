//! TH05 Item array discovery.
//!
//! [copy]
//!
//! Deleted that unnecersary check

/*
    TH05 Item Discovery of rrr.
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

pub fn item_array(mem: &mut ProcessMemory, player_pos: usize) -> Result<usize> {
    tracing::info!("Searching for item array via offset...");

    let candidate = (player_pos as isize + TH05Offsets::P2ITEMS) as usize;
    let data = mem.read(
        candidate,
        TH05ArrayLength::ITEM_COUNT * TH05Stride::ITEM_STRIDE,
    )?;

    let valid_item_types: [u8; 8] = [0, 1, 2, 3, 4, 5, 6, 0xFF];
    let checked = TH05ArrayLength::ITEM_COUNT.min(data.len() / TH05Stride::ITEM_STRIDE);
    let mut valid_count = 0usize;
    let mut active_count = 0usize;

    for i in 0..checked {
        let b = i * TH05Stride::ITEM_STRIDE;
        let flag = data[b];

        // Flag must be valid, no other flags are valid.
        if !matches!(flag, 0 | 1 | 2 | 3 | 0x80) {
            continue;
        }

        valid_count += 1;

        // Skip inactive items
        if flag == 0 {
            continue;
        }

        active_count += 1;

        // Verify item type
        let item_type = data[b + 14];
        if !valid_item_types.contains(&item_type) {
            continue;
        }

        // This one can really be negative compared to player because
        // it can flow out of the screen border but because I never opened PINCE
        // while playing it, idk if they really out and have negative
        let cur_x = i16::from_le_bytes([data[b + 2], data[b + 3]]);
        let cur_y = i16::from_le_bytes([data[b + 4], data[b + 5]]);

        if !(-32 * 16..=416 * 16).contains(&cur_x) || !(-48 * 16..=512 * 16).contains(&cur_y) {
            continue;
        }

        // Verify velocity is reasonable... but yep, that might be okay...
        let vel_x = i16::from_le_bytes([data[b + 10], data[b + 11]]);
        let vel_y = i16::from_le_bytes([data[b + 12], data[b + 13]]);

        if vel_x.abs() > 12 * 16 || vel_y.abs() > 12 * 16 {
            continue;
        }
    }

    // At least half of checked items should have valid flags
    if valid_count < checked / 2 {
        return Err(Error::ValidationFailed {
            reason: format!(
                "Item array validation failed: only {}/{} items have valid flags",
                valid_count, checked
            ),
        });
    }

    tracing::info!(
        "Found item array at 0x{:08X} (validated {}/{} items, {} active)",
        candidate,
        valid_count,
        checked,
        active_count
    );

    Ok(candidate)
}
