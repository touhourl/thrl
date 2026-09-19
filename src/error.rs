//! Error types for RL kit.
//! [copy]
//!
//! Also not changed, other than added a other error

/*
    Error handling of RL-rs, rrr.
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
use std::io;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Process {pid} not found")]
    ProcessNotFound { pid: i32 },

    #[error("Permission denied reading process {pid} memory")]
    PermissionDenied { pid: i32 },

    #[error("Failed to read memory at address 0x{address:08X}: {source}")]
    MemoryReadFailed {
        address: usize,
        #[source]
        source: io::Error,
    },

    #[error("Failed to parse /proc/{pid}/maps: {source}")]
    MapsParseError {
        pid: i32,
        #[source]
        source: io::Error,
    },

    #[error("Structure not found: {name}")]
    StructureNotFound { name: &'static str },

    #[error("Address validation failed: {reason}")]
    ValidationFailed { reason: String },

    #[error("Invalid game state: {reason}")]
    InvalidGameState { reason: String },

    #[error("dosbox-x process not found. Is dosbox-x running?")]
    DosboxXNotFound,

    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("TUI error: {0}")]
    Tui(String),

    #[error("Logging error: {0}")]
    Log(#[from] LogError),

    #[error("Invalid config: {0}")]
    InvalidConfig(String),

    /// We don't know why, it doesn't even matter how hard we try
    #[error("{context}: {source}")]
    WithContext {
        context: String,
        #[source]
        source: Box<Error>,
    },
}

impl Error {
    /// Add context to an error.
    pub fn context(self, context: impl Into<String>) -> Self {
        Error::WithContext {
            context: context.into(),
            source: Box::new(self),
        }
    }
}

#[derive(Error, Debug)]
pub enum LogError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("CSV error: {0}")]
    Csv(#[from] csv::Error),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Invalid path")]
    InvalidPath,

    #[error("Corrupted csv file")]
    CorruptedCSV,
}
