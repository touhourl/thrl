//! Low-level memory reading from /proc/PID/mem.
//! [copy]
//!
//! From last project. no changes.
//! Actually I wanna do like Linux permissions, drwx for all fn like
//! ru16le but from a reader, it is bad.
//! Since I have done it, I've never seen any issue about it like reading
//! issues, or sth like that. Having that I would doubt you need to install flex
//! and rebuild your kernel. Quite stable. It's just sits there and I even forgot it.

/*
    Memory helper of RL-rs.
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

use crate::error::{Error, Result};
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom};
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;

/// A memory region from /proc/<PID>/maps.
// (OFC, where can it else been? COM1?)
#[derive(Debug, Clone)]
pub struct MemoryRegion {
    pub start: usize,
    pub size: usize,
    /// Permission string ("rwx") etc.
    pub permissions: String,
    pub pathname: String,
}

impl MemoryRegion {
    #[inline]
    pub fn end(&self) -> usize {
        self.start + self.size
    }
    #[inline]
    pub fn is_readable(&self) -> bool {
        self.permissions.contains('r')
    }
    #[inline]
    pub fn is_writable(&self) -> bool {
        self.permissions.contains('w')
    }
    #[inline]
    pub fn is_private(&self) -> bool {
        self.permissions.contains('p')
    }
    #[inline]
    pub fn is_anonymous(&self) -> bool {
        self.pathname.is_empty()
    }
    #[inline]
    pub fn is_heap(&self) -> bool {
        self.pathname == "[heap]"
    }
    #[inline]
    pub fn contains(&self, addr: usize) -> bool {
        addr >= self.start && addr < self.end()
    }
}

pub struct ProcessMemory {
    pid: i32,
    mem_file: File,
    maps: Vec<MemoryRegion>,
}

impl ProcessMemory {
    /// Read from proc <PID> mem
    pub fn open(pid: i32) -> Result<Self> {
        let mem_path = format!("/proc/{}/mem", pid);
        let maps_path = format!("/proc/{}/maps", pid);

        // Check if process exists
        if !Path::new(&mem_path).exists() {
            return Err(Error::ProcessNotFound { pid });
        }

        // Load memory maps
        let maps = Self::load_maps(pid, &maps_path)?;

        let mem_file = OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(libc::O_LARGEFILE)
            .open(&mem_path)
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::PermissionDenied {
                    Error::PermissionDenied { pid }
                } else {
                    Error::Io(e)
                }
            })?;

        Ok(Self {
            pid,
            mem_file,
            maps,
        })
    }
    #[inline]
    pub fn pid(&self) -> i32 {
        self.pid
    }
    #[inline]
    pub fn maps(&self) -> &[MemoryRegion] {
        &self.maps
    }
    pub fn reload_maps(&mut self) -> Result<()> {
        let maps_path = format!("/proc/{}/maps", self.pid);
        self.maps = Self::load_maps(self.pid, &maps_path)?;
        Ok(())
    }
    fn load_maps(pid: i32, maps_path: &str) -> Result<Vec<MemoryRegion>> {
        let content = std::fs::read_to_string(maps_path)
            .map_err(|e| Error::MapsParseError { pid, source: e })?;

        let mut regions = Vec::new();

        for line in content.lines() {
            let p: Vec<&str> = line.split_whitespace().collect();
            if p.len() < 2 {
                continue;
            }
            let range = p[0];
            let Some((start, end)) = range.split_once('-') else {
                continue;
            };

            let Ok(start) = usize::from_str_radix(start, 16) else {
                continue;
            };
            let Ok(end) = usize::from_str_radix(end, 16) else {
                continue;
            };

            let permissions = p[1].to_string();
            let pathname = if p.len() >= 6 {
                p[5].to_string()
            } else {
                String::new()
            };

            regions.push(MemoryRegion {
                start,
                size: end - start,
                permissions,
                pathname,
            });
        }

        Ok(regions)
    }

    pub fn read_at(&mut self, address: usize, buf: &mut [u8]) -> Result<()> {
        self.mem_file
            .seek(SeekFrom::Start(address as u64))
            .map_err(|e| Error::MemoryReadFailed { address, source: e })?;

        self.mem_file
            .read_exact(buf)
            .map_err(|e| Error::MemoryReadFailed { address, source: e })?;

        Ok(())
    }

    /// Read memory at address, returming bytes.
    pub fn read(&mut self, address: usize, size: usize) -> Result<Vec<u8>> {
        let mut buf = vec![0u8; size];
        self.read_at(address, &mut buf)?;
        Ok(buf)
    }

    pub fn try_read(&mut self, address: usize, size: usize) -> Option<Vec<u8>> {
        self.read(address, size).ok()
    }
    // No docs for the fn below. nothing to say.

    pub fn read_u8(&mut self, address: usize) -> Result<u8> {
        let mut buf = [0u8; 1];
        self.read_at(address, &mut buf)?;
        Ok(buf[0])
    }
    pub fn read_u16_le(&mut self, address: usize) -> Result<u16> {
        let mut buf = [0u8; 2];
        self.read_at(address, &mut buf)?;
        Ok(u16::from_le_bytes(buf))
    }

    pub fn read_i16_le(&mut self, address: usize) -> Result<i16> {
        let mut buf = [0u8; 2];
        self.read_at(address, &mut buf)?;
        Ok(i16::from_le_bytes(buf))
    }

    pub fn read_u32_le(&mut self, address: usize) -> Result<u32> {
        let mut buf = [0u8; 4];
        self.read_at(address, &mut buf)?;
        Ok(u32::from_le_bytes(buf))
    }

    pub fn read_i32_le(&mut self, address: usize) -> Result<i32> {
        let mut buf = [0u8; 4];
        self.read_at(address, &mut buf)?;
        Ok(i32::from_le_bytes(buf))
    }

    pub fn write_at(&mut self, address: usize, buf: &[u8]) -> Result<()> {
        use std::io::Write;

        self.mem_file
            .seek(SeekFrom::Start(address as u64))
            .map_err(|e| Error::MemoryReadFailed { address, source: e })?;

        self.mem_file
            .write_all(buf)
            .map_err(|e| Error::MemoryReadFailed { address, source: e })?;

        Ok(())
    }

    pub fn write(&mut self, address: usize, data: &[u8]) -> Result<()> {
        self.write_at(address, data)
    }

    pub fn write_u8(&mut self, address: usize, value: u8) -> Result<()> {
        self.write_at(address, &[value])
    }

    pub fn write_u16_le(&mut self, address: usize, value: u16) -> Result<()> {
        self.write_at(address, &value.to_le_bytes())
    }

    pub fn write_i16_le(&mut self, address: usize, value: i16) -> Result<()> {
        self.write_at(address, &value.to_le_bytes())
    }

    pub fn write_u32_le(&mut self, address: usize, value: u32) -> Result<()> {
        self.write_at(address, &value.to_le_bytes())
    }

    pub fn write_i32_le(&mut self, address: usize, value: i32) -> Result<()> {
        self.write_at(address, &value.to_le_bytes())
    }

    pub fn search_pattern(
        &mut self,
        pattern: &[u8],
        regions: Option<&[MemoryRegion]>,
    ) -> Vec<usize> {
        let info: Vec<(usize, usize, bool)> = match regions {
            Some(r) => r
                .iter()
                .map(|reg| (reg.start, reg.size, reg.is_readable()))
                .collect(),
            None => self
                .maps
                .iter()
                .map(|reg| (reg.start, reg.size, reg.is_readable()))
                .collect(),
        };

        let mut matches = Vec::new();

        for (start, size, readable) in info {
            if !readable {
                continue;
            }

            if size > 10 * 1024 * 1024 {
                continue;
            }

            let Ok(data) = self.read(start, size) else {
                continue;
            };

            let mut offset = 0;
            while let Some(idx) = subsequence(&data[offset..], pattern) {
                matches.push(start + offset + idx);
                offset += idx + 1;
            }
        }

        matches
    }

    /// SELECT REGION FROM * WHERE F: Fn(&MemoryRegion) -> bool
    pub fn readable_regions<F>(&self, filter: F) -> Vec<&MemoryRegion>
    where
        F: Fn(&MemoryRegion) -> bool,
    {
        self.maps
            .iter()
            .filter(|r| r.is_readable() && filter(r))
            .collect()
    }

    pub fn data_regions(&self) -> Vec<&MemoryRegion> {
        self.readable_regions(|r| {
            r.is_writable()
                && r.is_private()
                && r.size > 0
                && r.size <= 8 * 1024 * 1024
                && !r.pathname.contains("[stack]")
                && !r.pathname.contains("memfd:pulseaudio")
        })
    }

    /// Get anonymous regions, the dosbox-x must live ib it.
    pub fn anonymous_regions(&self) -> Vec<&MemoryRegion> {
        self.readable_regions(|r| {
            r.is_writable()
                && r.is_private()
                && r.size > 0
                && r.size <= 8 * 1024 * 1024
                && (r.is_anonymous() || r.pathname.contains("dosbox-x"))
        })
    }
}

fn subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || needle.len() > haystack.len() {
        return None;
    }
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

pub fn find_pid() -> Result<i32> {
    use std::process::Command;

    let output = Command::new("pgrep")
        .args(["-f", "dosbox-x"])
        .output()
        .map_err(Error::Io)?;

    if !output.status.success() {
        return Err(Error::DosboxXNotFound);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout
        .lines()
        .next()
        .and_then(|s| s.trim().parse().ok())
        .ok_or(Error::DosboxXNotFound)
}
