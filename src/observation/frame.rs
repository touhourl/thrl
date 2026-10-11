use super::{schema0, schema1};
/*
    Frame of thrl observations.
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
pub enum Frame {
    Schema0(schema0::snapshot::Snapshot),
    Schema1(schema1::Frame),
}

impl Frame {
    pub fn history(&self) -> Self {
        match self {
            Self::Schema0(frame) => Self::Schema0(frame.history()),
            Self::Schema1(frame) => Self::Schema1(frame.history()),
        }
    }
}
