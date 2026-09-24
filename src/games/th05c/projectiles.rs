//! This file has no doccommits or anything else than this docstring. Why?
//! Because there is nothing to say.
//! Oh well I do have something to say: the laser and firewave and all the others
//! are likely all 0 while starting. But from our sight of memory address and map diff
//! they are totally correct. I wanna remove the validation but this will cause
//! my cloc count be somehow lower...

/*
    TH05 projectiles discovery of rrr.
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

pub fn laser_array(mem: &mut ProcessMemory, player_pos: usize) -> Result<usize> {
    tracing::info!("Searching for laser array via offset...");

    let candidate = (player_pos as isize + TH05COffsets::P2LASERS) as usize;
    let sample = mem.read(candidate, TH05CStride::LASER_STRIDE * 8)?;

    let mut valid = 0usize;
    let checked = 8.min(sample.len() / TH05CStride::LASER_STRIDE);

    for i in 0..checked {
        let base = i * TH05CStride::LASER_STRIDE;
        let flag = sample[base];

        if flag <= 7 {
            valid += 1;
        }
    }

    if valid < checked / 2 {
        return Err(Error::ValidationFailed {
            reason: format!(
                "Laser array validation failed: only {}/{} lasers are valid",
                valid, checked
            ),
        });
    }

    tracing::info!(
        "Found laser array at 0x{:08X} (validated {}/{})",
        candidate,
        valid,
        checked
    );

    Ok(candidate)
}

pub fn cheeto_trail_array(mem: &mut ProcessMemory, player_pos: usize) -> Result<usize> {
    tracing::info!("Searching for cheeto trail array via offset...");

    let candidate = (player_pos as isize + TH05COffsets::P2CHEETO) as usize;
    let _sample = mem.read(
        candidate,
        TH05CStride::CHEETO_STRIDE * TH05ArrayLength::CHEETO_TRAIL_COUNT,
    )?;

    tracing::info!("Found cheeto trail array at 0x{:08X}", candidate);

    Ok(candidate)
}

pub fn custom_entity_array(mem: &mut ProcessMemory, player_pos: usize) -> Result<usize> {
    tracing::info!("Searching for custom entity array via offset...");

    let candidate = (player_pos as isize + TH05COffsets::P2CE) as usize;
    let _sample = mem.read(candidate, TH05CStride::CE_STRIDE * 8)?;

    tracing::info!("Found custom entity array at 0x{:08X}", candidate);

    Ok(candidate)
}

pub fn firewave_array(mem: &mut ProcessMemory, player_pos: usize) -> Result<usize> {
    tracing::info!("Searching for firewave array via offset...");

    let candidate = (player_pos as isize + TH05COffsets::P2FIREWAVES) as usize;
    let _sample = mem.read(
        candidate,
        TH05CStride::FIREWAVE_STRIDE * TH05ArrayLength::FIREWAVE_COUNT,
    )?;

    tracing::info!("Found firewave array at 0x{:08X}", candidate);

    Ok(candidate)
}
