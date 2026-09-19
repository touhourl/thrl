//! Training data logging to CSV files. With training resume
//! and log overwrite.
//!
//! The last file was very messy and does not meet
//! production rules. Spagetti rust was it.
//!
//! [paper]
//! [copy]
//! [rewrite:80]
//! [basic]
//!
//! not used anymore ... i wanna delete it but we might have furture rewrite,
//! if the burn.rs has way better xpu support than vulkan / wgpu ... Imagine

/*
    Logging of RL-rs.
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
// Huh, typical markdown.
pub fn init_tracing() {
    use tracing_subscriber::EnvFilter;

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();
}
