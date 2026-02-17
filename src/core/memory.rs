use super::process::MemoryRegion;
use nix::errno::Errno;
use nix::sys::uio::{process_vm_readv, process_vm_writev, RemoteIoVec};
use nix::unistd::Pid;
use std::io::{self, IoSlice, IoSliceMut};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MemoryError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    #[error("Nix error: {0}")]
    Nix(#[from] nix::Error),
    #[error("Process error: {0}")]
    Process(#[from] super::process::ProcessError),
    #[error("Permission denied")]
    PermissionDenied,
    #[error("Invalid address")]
    InvalidAddress,
}

pub struct MemoryReader {
    pid: i32,
}

impl MemoryReader {
    pub fn new(pid: i32) -> Self {
        Self { pid }
    }

    /// Read memory from a specific address range
    pub fn read_memory(&self, address: usize, size: usize) -> Result<Vec<u8>, MemoryError> {
        let mut buffer = vec![0u8; size];

        let mut local_iov = [IoSliceMut::new(&mut buffer)];
        let remote_iov = [RemoteIoVec {
            base: address,
            len: size,
        }];

        let pid = Pid::from_raw(self.pid);

        match process_vm_readv(pid, &mut local_iov, &remote_iov) {
            Ok(bytes_read) => {
                if bytes_read == size {
                    Ok(buffer)
                } else {
                    buffer.truncate(bytes_read);
                    Ok(buffer)
                }
            }
            Err(Errno::EACCES) | Err(Errno::EPERM) => Err(MemoryError::PermissionDenied),
            Err(e) => Err(MemoryError::Nix(e)),
        }
    }

    /// Read memory region in chunks for better performance
    pub fn read_region(&self, region: &MemoryRegion) -> Result<Vec<u8>, MemoryError> {
        let size = region.size();
        self.read_memory(region.start, size)
    }

    /// Write memory to a specific address
    pub fn write_memory(&self, address: usize, data: &[u8]) -> Result<usize, MemoryError> {
        let local_iov = [IoSlice::new(data)];
        let remote_iov = [RemoteIoVec {
            base: address,
            len: data.len(),
        }];

        let pid = Pid::from_raw(self.pid);

        match process_vm_writev(pid, &local_iov, &remote_iov) {
            Ok(bytes_written) => Ok(bytes_written),
            Err(Errno::EACCES) | Err(Errno::EPERM) => Err(MemoryError::PermissionDenied),
            Err(e) => Err(MemoryError::Nix(e)),
        }
    }

    /// Read a value of specific size at address
    pub fn read_value(&self, address: usize, size: usize) -> Result<Vec<u8>, MemoryError> {
        self.read_memory(address, size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_reader_creation() {
        let reader = MemoryReader::new(1);
        assert_eq!(reader.pid, 1);
    }
}
