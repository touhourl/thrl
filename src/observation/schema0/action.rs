pub const ACTION_COUNT: usize = 20;

/*
    Taisei action indices of thrl.
    Copyright (C) 2026  T. Liu (touhourl@proton.me)

    This program is free software: you can redistribute it and/or modify
    it under the terms of the GNU Affero General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU Affero General Public License for more details.

    You should have received a copy of the GNU Affero General Public License
    along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/

const UP: u32 = 1 << 0;
const DOWN: u32 = 1 << 1;
const LEFT: u32 = 1 << 2;
const RIGHT: u32 = 1 << 3;
const FOCUS: u32 = 1 << 4;
const SHOT: u32 = 1 << 5;
const BOMB: u32 = 1 << 6;
const SPECIAL: u32 = 1 << 7;

/// Returns a u32 thing given a index, which could directly feed to taisei.
/// (shot is always on, the same as th04 and th05, only this time at thrl level)
pub fn buttons(index: usize) -> Option<u32> {
    let movement = match index % 9 {
        0 => 0,
        1 => LEFT,
        2 => RIGHT,
        3 => UP,
        4 => DOWN,
        5 => LEFT | UP,
        6 => LEFT | DOWN,
        7 => RIGHT | UP,
        8 => RIGHT | DOWN,
        _ => return None,
    };
    match index {
        0..=8 => Some(movement | SHOT),
        9..=17 => Some(movement | FOCUS | SHOT),
        18 => Some(BOMB | SHOT),
        19 => Some(SPECIAL | SHOT),
        _ => None,
    }
}
