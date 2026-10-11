use super::offsets::*;
pub use crate::games::th05c::key::{DiscreteAction, MoveAction};
/*
    Key input (th04A) of thrl.
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

/// Our hooks in ReC98.
pub fn key_det_player_pos(player_pos: usize) -> usize {
    player_pos - PLAYER + KEY_DET
}
/// This same, for th04.
pub fn shiftkey_player_pos(player_pos: usize) -> usize {
    player_pos - PLAYER + SHIFTKEY
}
