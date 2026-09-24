//! Unlike the th04 approach of last project, this one does
//! not use copy. This is because memory level control is more safe
//! and I don't need to run anymore in a VM, so I can also use my Intel
//! shitty gpu to train jt.
//! So, rewritten.
//!
//! [rewrite:90]
//!
//! Because only the structs are copied.
//! By the way, we don't have any dialogs !!!

/*
    TH05 keyboard control of rrr.
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
use super::offsets::TH05COffsets;
use crate::error::Result;
use crate::memory::ProcessMemory;

pub fn key_det_player_pos(player_pos: usize) -> usize {
    (player_pos as isize + TH05COffsets::P2KEY_DET) as usize
}

pub fn shiftkey_player_pos(player_pos: usize) -> usize {
    (player_pos as isize + TH05COffsets::P2SHIFT_KEY) as usize
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct KeyDet {
    pub value: u16,
}

impl KeyDet {
    pub fn read(mem: &mut ProcessMemory, key_det_addr: usize) -> Result<Self> {
        let value = mem.read_u16_le(key_det_addr)?;
        Ok(Self { value })
    }
    pub fn write(&self, mem: &mut ProcessMemory, key_det_addr: usize) -> Result<()> {
        mem.write_u16_le(key_det_addr, self.value)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ActionCommand {
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
    pub shift: bool,
    pub bomb: bool,
}

impl ActionCommand {
    /// Convert to key_det bit pattern.
    /// ```c
    /// ```
    pub fn to_key_det(&self) -> u16 {
        let mut bits = 0u16;

        if self.up {
            bits |= 0x0001;
        }
        if self.down {
            bits |= 0x0002;
        }
        if self.left {
            bits |= 0x0004;
        }
        if self.right {
            bits |= 0x0008;
        }
        if self.bomb {
            bits |= 0x0010;
        }

        // Our modified injection of extern bool shiftkey; /* ZUN symbol [MAGNet2010] */
        if self.shift {
            bits |= 0x8000;
        }

        bits
    }

    /// Apply this action command to game mem. Write key first, or a number,
    /// or the shift, or the binary. Then look at the key we will see it is applied.
    pub fn apply(
        &self,
        mem: &mut ProcessMemory,
        key_det_addr: usize,
        shiftkey_addr: usize,
    ) -> Result<()> {
        let key_det = self.to_key_det();
        mem.write_u16_le(key_det_addr, key_det)?;
        mem.write_u8(shiftkey_addr, if self.shift { 1 } else { 0 })?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveAction {
    Idle,
    Left,
    Right,
    Up,
    Down,
    LeftUp,
    LeftDown,
    RightUp,
    RightDown,
}

impl MoveAction {
    /// Total number of movement actions.
    pub const fn size() -> usize {
        9
    }

    /// Convert from index (0-8).
    pub fn from_index(idx: usize) -> Option<Self> {
        match idx {
            0 => Some(Self::Idle),
            1 => Some(Self::Left),
            2 => Some(Self::Right),
            3 => Some(Self::Up),
            4 => Some(Self::Down),
            5 => Some(Self::LeftUp),
            6 => Some(Self::LeftDown),
            7 => Some(Self::RightUp),
            8 => Some(Self::RightDown),
            _ => None,
        }
    }

    /// Convert to index (0-8).
    pub fn to_index(self) -> usize {
        match self {
            Self::Idle => 0,
            Self::Left => 1,
            Self::Right => 2,
            Self::Up => 3,
            Self::Down => 4,
            Self::LeftUp => 5,
            Self::LeftDown => 6,
            Self::RightUp => 7,
            Self::RightDown => 8,
        }
    }

    /// Convert to action command.
    pub fn to_command(self) -> ActionCommand {
        match self {
            Self::Idle => ActionCommand::default(),
            Self::Left => ActionCommand {
                left: true,
                ..ActionCommand::default()
            },
            Self::Right => ActionCommand {
                right: true,
                ..ActionCommand::default()
            },
            Self::Up => ActionCommand {
                up: true,
                ..ActionCommand::default()
            },
            Self::Down => ActionCommand {
                down: true,
                ..ActionCommand::default()
            },
            Self::LeftUp => ActionCommand {
                left: true,
                up: true,
                ..ActionCommand::default()
            },
            Self::LeftDown => ActionCommand {
                left: true,
                down: true,
                ..ActionCommand::default()
            },
            Self::RightUp => ActionCommand {
                right: true,
                up: true,
                ..ActionCommand::default()
            },
            Self::RightDown => ActionCommand {
                right: true,
                down: true,
                ..ActionCommand::default()
            },
        }
    }
}

/// Discrete action::19 suitable for PPO sampling.
///
/// Total size is 9 (movement) * 2 (shift) + 1 (bomb) = 19.
/// But, we might have used the IDLE also as shift, even though I don't think it is a bug.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiscreteAction {
    Move { movement: MoveAction, shift: bool },
    Bomb,
}

impl DiscreteAction {
    /// Total number of discrete actions.
    pub const fn size() -> usize {
        MoveAction::size() * 2 + 1
    }

    /// Deterministic index mapping:
    /// `idx = movement + shift*9`, and `idx = 18` for bomb.
    pub fn from_index(idx: usize) -> Option<Self> {
        let move_size = MoveAction::size();
        if idx < move_size * 2 {
            let movement = MoveAction::from_index(idx % move_size)?;
            let shift = (idx / move_size) == 1;
            Some(Self::Move { movement, shift })
        } else if idx == move_size * 2 {
            Some(Self::Bomb)
        } else {
            None
        }
    }

    /// Convert to index (0-18).
    pub fn to_index(self) -> usize {
        let move_size = MoveAction::size();
        match self {
            Self::Move { movement, shift } => {
                let shift_idx = if shift { 1 } else { 0 };
                movement.to_index() + shift_idx * move_size
            }
            Self::Bomb => move_size * 2,
        }
    }

    /// Convert to action command.
    pub fn to_command(self) -> ActionCommand {
        match self {
            Self::Move { movement, shift } => {
                let mut cmd = movement.to_command();
                cmd.shift = shift;
                cmd
            }
            Self::Bomb => ActionCommand {
                bomb: true,
                ..ActionCommand::default()
            },
        }
    }

    /// Apply this discrete action to game memory.
    pub fn apply(
        &self,
        mem: &mut ProcessMemory,
        key_det_addr: usize,
        shiftkey_addr: usize,
    ) -> Result<()> {
        self.to_command().apply(mem, key_det_addr, shiftkey_addr)
    }
}
