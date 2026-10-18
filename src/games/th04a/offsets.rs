//! So, thanks to the ReC98 here.
//!
//! Last day I was just cloning ReC98 and want to do some changes. I really want
//! to contribute. So, occasionally I just see the
//! ```asm
//! aWitchingDream	db 'Witching Dream',0
//! aSeleneSLight	db 'Selene',27h,'s light',0
//! ```
//! Em. using them might be an option to do it and therefore I also avoided
//! using the old method of th05c, which hardcodes the offset.
//!
//! th05c now technical debts; I will fix it sometime but you must know that thrl is 
//! distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without 
//! even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
//!
//! Therefore, I recommend using th04. Also th05 will likely rescieve
//! a better version, too.
//!
//! Those values are calculated directly from .map, cxx. Not from anywhere.
//! You could see repo th98patch and verify yourself.

/*
    Offsets (th04A) of thrl.
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

pub const DATA_SEGMENT: usize = 0x1BF4;
pub const DATA_SIGNATURE: &[u8] = b"Witching Dream\0Selene's light\0";
pub const DATA_SIGNATURE_OFFSET: usize = 0x1D33;
pub const RESIDENT: usize = 0xB896;
pub const PLAYER: usize = 0x445E;
pub const POWER: usize = 0x4474;
pub const INVINCIBILITY: usize = 0x4472;
pub const MISS_TIME: usize = 0x447A;
pub const BOMBING: usize = 0x4178;
pub const KEY_DET: usize = 0x3784;
pub const SHIFTKEY: usize = 0x3786;
pub const BULLETS: usize = 0x5832;
pub const BULLET_COUNT: usize = 440;
pub const BULLET_STRIDE: usize = 26;
pub const ENEMIES: usize = 0x88A2;
pub const ENEMY_COUNT: usize = 32;
pub const ENEMY_STRIDE: usize = 64;
pub const ITEMS: usize = 0xAD44;
pub const ITEM_COUNT: usize = 32;
pub const ITEM_STRIDE: usize = 20;
pub const BOSS: usize = 0x51DA;
pub const MIDBOSS: usize = 0x51C4;
pub const MIDBOSS_ACTIVE: usize = 0x44C2;
pub const THICKLASERS: usize = 0x4100;
pub const THICKLASER_STRIDE: usize = 24;
pub const SCORE: usize = 0x4159;
pub const STAGE_GRAZE: usize = 0xBACC;
pub const STAGE_POINT: usize = 0x447B;
pub const DREAM_ITEMS: usize = 0x4486;
pub const DREAM_SCORE: usize = 0xBADC;
pub const PLAYER_IS_HIT: usize = 0x4479;
pub const CUSTOM_ENTITIES: usize = 0xB014;
pub const CUSTOM_STRIDE: usize = 26;
pub const SCYTHE_ACTIVE: usize = 0x44F7;
pub const SCYTHE_MOTION: usize = 0x44F8;
