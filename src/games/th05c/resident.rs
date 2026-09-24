//! Resident structure finder.

use crate::error::{Error, Result};
use crate::games::th05c::offsets;
use crate::memory::ProcessMemory;
#[allow(unused)]
const SCORE_DIGITS: usize = 8; //Counted by me
const MAIN_STAGE_COUNT: usize = 6;
const _STAGE_EXTRA: usize = MAIN_STAGE_COUNT;

/*
    TH05 Resident structure finder of rrr.
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

// [copy]
//
// th04/score.h

// [copy]
//
// th05/resident.hpp

/// Find resident structure by searching for `KSOConfig` signature.
/// In th04, it is `HUMAConfig`. I will consider make it a common later but not now.
pub fn find_resident(mem: &mut ProcessMemory) -> Result<usize> {
    tracing::info!("Searching for resident structure...");

    let matches = mem.search_pattern(offsets::TH05Config::RESIDENT_SIGNATURE, None);

    if matches.is_empty() {
        return Err(Error::StructureNotFound { name: "resident" });
    }

    tracing::debug!("Found {} potential matches", matches.len());

    for addr in matches {
        if let Ok(data) = mem.read(addr, 100) {
            tracing::debug!(
                "Checking address 0x{:08X}: bytes [9-20] = {:?}",
                addr,
                &data[9..21]
            );

            let cfg_power = data[12];
            let credit_lives = data[13];
            let credit_bombs = data[14];
            let cfg_lives = data[15];
            let cfg_bombs = data[16];
            let rank = data[17];
            let _bgm_mode = data[18];
            let stage = data[19];

            tracing::debug!(
                "Validation: cfg_power={}, credit_lives={}, credit_bombs={}, cfg_lives={}, cfg_bombs={}, rank={}, stage={}",
                cfg_power,
                credit_lives,
                credit_bombs,
                cfg_lives,
                cfg_bombs,
                rank,
                stage
            );

            // Here, the credit lives and bombs can theoretically be 0xFF. But I doubt
            // someone will actually do so...
            if cfg_power <= 128
                && credit_lives <= 99
                && credit_bombs <= 99
                && cfg_lives <= 99
                && cfg_bombs <= 99
                && rank <= 3
                && stage <= MAIN_STAGE_COUNT as u8
            {
                tracing::info!(
                    "Found valid resident at 0x{:08X} (cfg_power={}, credit_lives={}, credit_bombs={}, rank={}, stage={})",
                    addr,
                    cfg_power,
                    credit_lives,
                    credit_bombs,
                    rank,
                    stage
                );
                return Ok(addr);
            }
        }
    }

    Err(Error::StructureNotFound { name: "resident" })
}
