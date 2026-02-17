use super::memory::{MemoryError, MemoryReader};
use super::process::get_memory_regions;
use super::types::{FoundAddress, ScanType, Value, ValueType};
use rayon::prelude::*;
use std::sync::atomic::{AtomicUsize, Ordering};

pub struct Scanner {
    pid: i32,
    reader: MemoryReader,
    value_type: ValueType,
}

impl Scanner {
    pub fn new(pid: i32, value_type: ValueType) -> Self {
        Self {
            pid,
            reader: MemoryReader::new(pid),
            value_type,
        }
    }

    /// Perform initial scan and stream incremental batches with progress updates.
    pub fn initial_scan_streaming_with_progress(
        &self,
        scan_type: ScanType,
        target_value: Option<Value>,
        batch_size: usize,
        sender: std::sync::mpsc::Sender<Vec<FoundAddress>>,
        progress_sender: std::sync::mpsc::Sender<(usize, usize)>,
    ) -> Result<(), MemoryError> {
        let regions = get_memory_regions(self.pid)?;
        eprintln!(
            "[scan] initial_scan_streaming_with_progress pid={} scan_type={:?} value_type={:?} regions={}",
            self.pid,
            scan_type,
            self.value_type,
            regions.len()
        );

        let writable_regions: Vec<_> = regions
            .into_iter()
            .filter(|r| r.writable && r.readable)
            .collect();

        let total_bytes: usize = writable_regions.iter().map(|r| r.size()).sum();
        let scanned_bytes = AtomicUsize::new(0);
        let _ = progress_sender.send((0, total_bytes));

        let value_size = self.value_type.size();
        let batch_size = batch_size.max(1);

        writable_regions.par_iter().for_each(|region| {
            let sender = sender.clone();
            let progress_sender = progress_sender.clone();
            match self.reader.read_region(region) {
                Ok(data) => {
                    let mut local_results = Vec::new();
                    let mut offset = 0;
                    while offset + value_size <= data.len() {
                        if let Some(current_value) =
                            Value::from_bytes(&data[offset..offset + value_size], self.value_type)
                        {
                            let matches = match scan_type {
                                ScanType::ExactValue => {
                                    if let Some(ref target) = target_value {
                                        current_value.compare(target, ScanType::ExactValue)
                                    } else {
                                        false
                                    }
                                }
                                ScanType::GreaterThan => {
                                    if let Some(ref target) = target_value {
                                        current_value.compare(target, ScanType::GreaterThan)
                                    } else {
                                        false
                                    }
                                }
                                ScanType::LessThan => {
                                    if let Some(ref target) = target_value {
                                        current_value.compare(target, ScanType::LessThan)
                                    } else {
                                        false
                                    }
                                }
                                ScanType::UnknownInitial => true,
                                _ => false,
                            };

                            if matches {
                                local_results.push(FoundAddress {
                                    address: region.start + offset,
                                    value: current_value,
                                });

                                if local_results.len() >= batch_size {
                                    let chunk: Vec<FoundAddress> =
                                        local_results.drain(..).collect();
                                    let _ = sender.send(chunk);
                                }
                            }
                        }
                        offset += 1;
                    }

                    if !local_results.is_empty() {
                        let _ = sender.send(local_results);
                    }
                }
                Err(err) => {
                    eprintln!(
                        "[scan] read_region failed pid={} region={:016X}-{:016X} perms=r{}w{}x{} err={}",
                        self.pid,
                        region.start,
                        region.end,
                        if region.readable { "1" } else { "0" },
                        if region.writable { "1" } else { "0" },
                        if region.executable { "1" } else { "0" },
                        err
                    );
                }
            }

            let region_size = region.size();
            let scanned = scanned_bytes.fetch_add(region_size, Ordering::Relaxed) + region_size;
            let _ = progress_sender.send((scanned, total_bytes));
        });

        Ok(())
    }

    /// Rescan existing addresses with new criteria
    pub fn next_scan(
        &self,
        addresses: &[FoundAddress],
        scan_type: ScanType,
        target_value: Option<Value>,
    ) -> Result<Vec<FoundAddress>, MemoryError> {
        let results: Vec<FoundAddress> = addresses
            .par_iter()
            .filter_map(|found| {
                let value_size = self.value_type.size();
                match self.reader.read_value(found.address, value_size) {
                    Ok(data) => {
                        if let Some(current_value) = Value::from_bytes(&data, self.value_type) {
                            let matches =
                                match scan_type {
                                    ScanType::ExactValue => {
                                        if let Some(ref target) = target_value {
                                            current_value.compare(target, ScanType::ExactValue)
                                        } else {
                                            false
                                        }
                                    }
                                    ScanType::GreaterThan => {
                                        if let Some(ref target) = target_value {
                                            current_value.compare(target, ScanType::GreaterThan)
                                        } else {
                                            false
                                        }
                                    }
                                    ScanType::LessThan => {
                                        if let Some(ref target) = target_value {
                                            current_value.compare(target, ScanType::LessThan)
                                        } else {
                                            false
                                        }
                                    }
                                    ScanType::IncreasedValue => current_value
                                        .compare(&found.value, ScanType::IncreasedValue),
                                    ScanType::DecreasedValue => current_value
                                        .compare(&found.value, ScanType::DecreasedValue),
                                    ScanType::ChangedValue => {
                                        current_value.compare(&found.value, ScanType::ChangedValue)
                                    }
                                    ScanType::UnchangedValue => current_value
                                        .compare(&found.value, ScanType::UnchangedValue),
                                    ScanType::UnknownInitial => true,
                                };

                            if matches {
                                return Some(FoundAddress {
                                    address: found.address,
                                    value: current_value,
                                });
                            }
                        }
                    }
                    Err(err) => {
                        eprintln!(
                            "[scan] read_value failed pid={} address={:016X} size={} err={}",
                            self.pid, found.address, value_size, err
                        );
                    }
                }
                None
            })
            .collect();

        Ok(results)
    }

    /// Read current value at an address
    pub fn read_address(&self, address: usize) -> Result<Value, MemoryError> {
        let value_size = self.value_type.size();
        let data = self.reader.read_value(address, value_size)?;
        Value::from_bytes(&data, self.value_type).ok_or(MemoryError::InvalidAddress)
    }

    /// Write a value to an address
    pub fn write_address(&self, address: usize, value: &Value) -> Result<(), MemoryError> {
        let bytes = value.to_bytes();
        self.reader.write_memory(address, &bytes)?;
        Ok(())
    }
}
