use super::offsets::*;
use crate::error::Result;
use crate::memory::ProcessMemory;

/*
    TH05 enemy discovery of rrr.
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
pub fn enemy_array(mem: &mut ProcessMemory, bullets_addr: usize) -> Result<usize> {
    tracing::info!("Searching for enemy array via offset...");

    // Calc the thing directly using +
    let candidate = bullets_addr + TH05Offsets::B2ENEMIES;
    let data = mem.read(candidate, TH05Stride::ENEMY_STRIDE * 8)?;

    let mut valid = 0usize;
    let checked = 8.min(data.len() / TH05Stride::ENEMY_STRIDE);
    // we still use from le bytes cuz we ain't sure other than playfieldmotions
    for i in 0..checked {
        let b = i * TH05Stride::ENEMY_STRIDE;
        let flag = data[b];
        let age = data[b + 1];
        let hp = i16::from_le_bytes([data[b + 14], data[b + 15]]);
        // A silly match. But I can confirm it should be correct even it is stupid
        // because if we just got the P2resident correct.
        if matches!(flag, 0..=3) && age < 240 && (-1..=30000).contains(&hp) {
            valid += 1;
        }
    }

    tracing::info!(
        "Found enemy array at 0x{:08X} (validated {}/{})",
        candidate,
        valid,
        checked
    );

    Ok(candidate)
}
