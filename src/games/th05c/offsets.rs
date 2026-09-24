pub struct TH05COffsets;

/*
    Offsets of TH05 of rrr.
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

/// You might want to see the .map files in another repo.
/// the R means resident, P means player, B means bullets and E means enemy.
impl TH05COffsets {
    /// Runtime offset: resident structure base (KSOConfig) to player_pos.
    /// This is the actual distance in memory from the resident structure to player_pos.
    /// It is **not** pointer, I swear.
    ///
    /// [bug]
    ///
    /// To say it's a bug, it is but I can say it changes and might you
    /// need to open PINCE and cheat engine the player_pos and calculate the
    /// offset by yourself. Warning here about "It works on my machine".
    ///
    /// There is current no useful ways to calculate the real address single by the KSOConfig.
    /// Unless there is a flag there. I only know to look at map file and might has relationship
    /// with the memory of debloatm and debloat, maybe total?
    pub const R2PLAYER: isize = -0x67674;
    // === Can be calculated by map === \\
    pub const P2BOSS_POS: isize = -0x6B6A;
    pub const P2BOSS_HP: isize = -0x6B5E;
    pub const P2BOSS2_POS: isize = -0x6B52;
    pub const P2BOSS2_HP: isize = -0x6B46;
    pub const P2MIDBOSS_POS: isize = -0x6B80; //0x5AAC - 0xC62C
    pub const P2MIDBOSS_HP: isize = -0x6B72; //0x5ABA - 0xC62C
    pub const P2BULLETS: isize = -0x66FA;
    pub const P2ENEMIES: isize = -0x322A;
    pub const P2ITEMS: isize = -0x15A0;
    pub const P2STAGE_GRAZE: isize = -0x526;
    pub const P2INVINCIBLE: usize = 0x1C;
    pub const P2KEY_DET: isize = -0x9448;
    pub const P2SHIFT_KEY: isize = -0x9446;
    pub const P2POWER: usize = 0x1E;
    pub const P2SHOT_LEVEL: usize = 0x1F;
    pub const P2SHOT_TIME: usize = 0x20;
    pub const P2DREAM: usize = 0x23;
    pub const P2STAGE_POINT: usize = 0x26;
    /// Bullet to enemy! not bomb...
    pub const B2ENEMIES: usize = 0x34D0;
    pub const E2ITEMS: usize = 0x1C8A;
    // ===Projectile arrays (Lasers, Cheetos, CEs)=== \\
    /// Lasers are used by various bosses for attacks.
    /// Examples are ExAlice, 03 double.
    pub const P2LASERS: isize = -0x6E98;

    /// Cheeto leave trails of nodes behind them, used by ExAlice and ShinkiHpT
    pub const P2CHEETO: isize = -0x4FE;
    /// Shared pool: b6balls, swords, cheeto heads, etc.
    /// Smart and thank you ZUN. It simplifies our program by 0.1 %.
    /// But why don't you move some of them to bullets???
    pub const P2CE: isize = -0x1230;
    /// Firewaves are areas used in ExAlice.
    /// She is the most difficult boss I ever seen. I died 20 times
    /// while testing it.
    pub const P2FIREWAVES: isize = -0x48;
}

pub struct TH05CStride;
/// They are strides in bytes, not bit nor any other things like int or bool...
impl TH05CStride {
    /// th05 has 6 extra bytes compared to th04.
    pub const BULLET_STRIDE: usize = 32;
    pub const ENEMY_STRIDE: usize = 64;
    pub const ITEM_STRIDE: usize = 20;
    pub const LASER_STRIDE: usize = 24;
    pub const CHEETO_STRIDE: usize = 82;
    pub const CE_STRIDE: usize = 26;
    pub const FIREWAVE_STRIDE: usize = 6;
}

pub struct TH05Config;

impl TH05Config {
    pub const PLAYFIELD_W: f32 = 384.0;
    pub const PLAYFIELD_H: f32 = 368.0; // Actual playfield height (not 480!)
    pub const POWER_MAX: u8 = 128;
    pub const MAX_LIVES: u8 = 6;
    pub const MAX_BOMBS: u8 = 5;
    pub const MAX_STAGE: u8 = 6;
    pub const MAX_RANK: u8 = 3;

    /// Resident_t id signature. Subject to be used as common file
    pub const RESIDENT_SIGNATURE: &'static [u8] = b"KSOConfig";
}

pub struct TH05ArrayLength;
impl TH05ArrayLength {
    pub const BULLET_COUNT: usize = 397;
    pub const ENEMY_COUNT: usize = 320;
    pub const ITEM_COUNT: usize = 248;
    pub const LASER_COUNT: usize = 32;
    pub const CHEETO_TRAIL_COUNT: usize = 8;
    pub const CUSTOM_COUNT: usize = 64;
    pub const FIREWAVE_COUNT: usize = 2;
}
