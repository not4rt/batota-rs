use std::fs;
use std::io;
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ProcessError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
}

#[derive(Debug, Clone)]
pub struct Process {
    pub pid: i32,
    pub name: String,
}

impl Process {
    pub fn new(pid: i32, name: String) -> Self {
        Self { pid, name }
    }
}

#[derive(Debug, Clone)]
pub struct MemoryRegion {
    pub start: usize,
    pub end: usize,
    pub readable: bool,
    pub writable: bool,
    pub executable: bool,
    #[allow(dead_code)]
    pub offset: usize,
    #[allow(dead_code)]
    pub pathname: String,
}

impl MemoryRegion {
    pub fn size(&self) -> usize {
        self.end - self.start
    }
}

pub fn list_processes() -> Result<Vec<Process>, ProcessError> {
    let mut processes = Vec::new();

    for entry in fs::read_dir("/proc")? {
        let entry = entry?;
        let filename = entry.file_name();
        let filename_str = filename.to_string_lossy();

        // Check if directory name is a number (PID)
        if let Ok(pid) = filename_str.parse::<i32>() {
            // Read process name from /proc/[pid]/comm
            let comm_path = format!("/proc/{}/comm", pid);
            if let Ok(name) = fs::read_to_string(&comm_path) {
                processes.push(Process::new(pid, name.trim().to_string()));
            }
        }
    }

    processes.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(processes)
}

pub fn get_memory_regions(pid: i32) -> Result<Vec<MemoryRegion>, ProcessError> {
    let maps_path = format!("/proc/{}/maps", pid);
    let maps_content = fs::read_to_string(&maps_path)?;

    let mut regions = Vec::new();

    for line in maps_content.lines() {
        if let Some(region) = parse_maps_line(line) {
            // Include all readable AND writable regions
            // This is what we need to scan for values
            if region.readable && region.writable {
                regions.push(region);
            }
        }
    }

    Ok(regions)
}

fn parse_maps_line(line: &str) -> Option<MemoryRegion> {
    // Format: address perms offset dev inode pathname
    // Example: 00400000-00452000 r-xp 00000000 08:02 173521      /usr/bin/dbus-daemon
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 5 {
        return None;
    }

    // Parse address range
    let addr_parts: Vec<&str> = parts[0].split('-').collect();
    if addr_parts.len() != 2 {
        return None;
    }

    let start = usize::from_str_radix(addr_parts[0], 16).ok()?;
    let end = usize::from_str_radix(addr_parts[1], 16).ok()?;

    // Parse permissions
    let perms = parts[1];
    let readable = perms.chars().nth(0)? == 'r';
    let writable = perms.chars().nth(1)? == 'w';
    let executable = perms.chars().nth(2)? == 'x';

    // Parse offset
    let offset = usize::from_str_radix(parts[2], 16).ok()?;

    // Parse pathname (if exists)
    let pathname = if parts.len() > 5 {
        parts[5..].join(" ")
    } else {
        String::new()
    };

    Some(MemoryRegion {
        start,
        end,
        readable,
        writable,
        executable,
        offset,
        pathname,
    })
}

pub fn check_process_exists(pid: i32) -> bool {
    Path::new(&format!("/proc/{}", pid)).exists()
}
