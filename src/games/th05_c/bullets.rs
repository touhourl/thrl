use super::offsets::*;
use crate::error::{Error, Result};
use crate::memory::ProcessMemory;

/*
    TH05 Bullets discovery of rrr.
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
pub fn bullet_array(mem: &mut ProcessMemory, player_pos: usize) -> Result<usize> {
    tracing::info!("Searching for bullet array via offset...");

    let candidate = (player_pos as isize + TH05Offsets::P2BULLETS) as usize;
    let sample = mem.read(candidate, TH05Stride::BULLET_STRIDE * 64)?;

    let mut valid = 0usize;
    let checked = 64.min(sample.len() / TH05Stride::BULLET_STRIDE);

    for i in 0..checked {
        let base = i * TH05Stride::BULLET_STRIDE;
        let flag = sample[base];
        let age = sample[base + 1];
        let spawn_flag = sample[base + 18];
        let move_flag = sample[base + 19];

        if matches!(flag, 0 | 1 | 2 | 3 | 0x80) && age < 240 && spawn_flag <= 64 && move_flag <= 64
        {
            valid += 1;
        }
    }

    // At least 1/3 of checked bullets should be valid
    if valid < 8.max(checked / 3) {
        return Err(Error::ValidationFailed {
            reason: format!(
                "Bullet array validation failed: only {}/{} bullets are valid",
                valid, checked
            ),
        });
    }

    tracing::info!(
        "Found bullet array at 0x{:08X} (validated {}/{})",
        candidate,
        valid,
        checked
    );

    Ok(candidate)
}
