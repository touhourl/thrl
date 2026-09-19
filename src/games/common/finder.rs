// use crate::error::Result;
// use crate::memory::ProcessMemory;


/*
    Memory finders of rrr.
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

/// Discovered memory addresses.
#[derive(Debug, Clone, Default)]
pub struct DiscoveredAddresses {
    pub resident: Option<usize>,
    pub player_pos: Option<usize>,
    pub bullets: Option<usize>,
    pub enemies: Option<usize>,
    pub items: Option<usize>,
    pub score: Option<usize>,
    pub power: Option<usize>,
    pub midboss: Option<usize>,
    pub midboss_hp: Option<usize>,
    pub boss: Option<usize>,
    pub boss_hp: Option<usize>,
    pub boss_2: Option<usize>,
    pub boss_2_hp: Option<usize>,
    pub stage_point: Option<usize>,
    /// Not present in th02.
    pub stage_graze: Option<usize>,
    /// Have different meaning in different games.
    pub dream_score: Option<usize>,
    pub key_det: Option<usize>,
}
