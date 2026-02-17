use eframe::egui;
use std::sync::mpsc::{channel, Receiver};
use std::thread;
use std::time::{Duration, Instant};

use super::tables::{FoundAddressesTable, SavedAddress as TableSavedAddress, SavedAddressesTable};
use crate::core::{
    list_processes, FoundAddress, MemoryReader, Process, ScanType, Scanner, Value, ValueType,
};

pub(crate) struct CheatEngineAppState {
    pub(crate) processes: Vec<Process>,
    pub(crate) selected_process: Option<Process>,
    pub(crate) show_process_list: bool,
    pub(crate) process_filter: String,

    pub(crate) value_type: ValueType,
    pub(crate) scan_type: ScanType,
    pub(crate) search_value: String,

    pub(crate) scan_results: Vec<FoundAddress>,
    pub(crate) saved_addresses: Vec<SavedAddress>,
    pub(crate) is_scanning: bool,
    pub(crate) scan_receiver: Option<Receiver<ScanResult>>,
    pub(crate) scan_stream_receiver: Option<Receiver<Vec<FoundAddress>>>,
    pub(crate) scan_done_receiver: Option<Receiver<Result<(), String>>>,
    pub(crate) scan_progress_receiver: Option<Receiver<(usize, usize)>>,
    pub(crate) scan_progress_current: usize,
    pub(crate) scan_progress_total: usize,
    pub(crate) first_scan: bool,

    pub(crate) selected_result_indices: Vec<usize>,
    pub(crate) selected_saved_indices: Vec<usize>,
    pub(crate) status_message: String,
    pub(crate) scroll_to_top: bool,
    pub(crate) value_edit_buffers: Vec<String>,
    pub(crate) editing_saved_value: Option<usize>,
    pub(crate) scan_value_type: ValueType,
    pub(crate) last_refresh: Instant,
    pub(crate) refresh_interval: Duration,
}

pub(crate) enum ScanResult {
    Complete(Vec<FoundAddress>),
    Error(String),
}

#[derive(Clone)]
pub(crate) struct SavedAddress {
    pub(crate) address: usize,
    pub(crate) description: String,
    pub(crate) value: Value,
    pub(crate) value_type: ValueType,
    pub(crate) frozen: bool,
}

impl Default for CheatEngineAppState {
    fn default() -> Self {
        Self {
            processes: Vec::new(),
            selected_process: None,
            show_process_list: false,
            process_filter: String::new(),
            value_type: ValueType::I32,
            scan_type: ScanType::ExactValue,
            search_value: String::new(),
            scan_results: Vec::new(),
            saved_addresses: Vec::new(),
            is_scanning: false,
            scan_receiver: None,
            scan_stream_receiver: None,
            scan_done_receiver: None,
            scan_progress_receiver: None,
            scan_progress_current: 0,
            scan_progress_total: 0,
            first_scan: true,
            selected_result_indices: Vec::new(),
            selected_saved_indices: Vec::new(),
            status_message: "Ready".to_string(),
            scroll_to_top: false,
            value_edit_buffers: Vec::new(),
            editing_saved_value: None,
            scan_value_type: ValueType::I32,
            last_refresh: Instant::now(),
            refresh_interval: Duration::from_millis(250),
        }
    }
}

impl CheatEngineAppState {
    pub(crate) fn apply_ce_style(&self, ctx: &egui::Context) {
        let mut style = (*ctx.style()).clone();
        style.spacing.item_spacing = egui::vec2(6.0, 4.0);
        style.spacing.window_margin = egui::Margin::same(6);
        ctx.set_style(style);
    }

    pub(crate) fn open_process_list(&mut self) {
        match list_processes() {
            Ok(procs) => {
                self.processes = procs;
                self.show_process_list = true;
            }
            Err(e) => self.status_message = format!("Error: {}", e),
        }
    }

    pub(crate) fn start_scan(&mut self, ctx: &egui::Context) {
        if self.selected_process.is_none() {
            self.status_message = "No process selected".to_string();
            return;
        }

        let process = self.selected_process.as_ref().unwrap();
        let pid = process.pid;
        let value_type = self.value_type;
        let scan_type = self.scan_type;

        let target_value = if matches!(
            scan_type,
            ScanType::UnknownInitial
                | ScanType::IncreasedValue
                | ScanType::DecreasedValue
                | ScanType::ChangedValue
                | ScanType::UnchangedValue
        ) {
            None
        } else {
            match self.parse_value() {
                Ok(v) => Some(v),
                Err(e) => {
                    self.status_message = format!("Invalid value: {}", e);
                    return;
                }
            }
        };

        let (batch_tx, batch_rx) = channel();
        let (done_tx, done_rx) = channel();
        let (progress_tx, progress_rx) = channel();
        self.scan_receiver = None;
        self.scan_stream_receiver = Some(batch_rx);
        self.scan_done_receiver = Some(done_rx);
        self.scan_progress_receiver = Some(progress_rx);
        self.scan_progress_current = 0;
        self.scan_progress_total = 0;

        let existing_results = if self.first_scan {
            Vec::new()
        } else {
            self.scan_results.clone()
        };

        let is_first_scan = self.first_scan;

        self.is_scanning = true;
        self.scan_results.clear();
        self.selected_result_indices.clear();
        let value_str = if self.search_value.trim().is_empty() {
            "-".to_string()
        } else {
            self.search_value.trim().to_string()
        };
        self.status_message = format!(
            "Scanning pid={} type={} scan={} value={}",
            pid, value_type, scan_type, value_str
        );
        self.scroll_to_top = true;

        thread::spawn(move || {
            let scanner = Scanner::new(pid, value_type);

            if is_first_scan {
                let batch_size = 512;
                match scanner.initial_scan_streaming_with_progress(
                    scan_type,
                    target_value,
                    batch_size,
                    batch_tx,
                    progress_tx,
                ) {
                    Ok(_) => {
                        let _ = done_tx.send(Ok(()));
                    }
                    Err(e) => {
                        let _ = done_tx.send(Err(format!("{}", e)));
                    }
                }
            } else {
                match scanner.next_scan(&existing_results, scan_type, target_value) {
                    Ok(addrs) => {
                        let _ = batch_tx.send(addrs);
                        let _ = done_tx.send(Ok(()));
                    }
                    Err(e) => {
                        let _ = done_tx.send(Err(format!("{}", e)));
                    }
                }
            }
        });
    }

    pub(crate) fn parse_value(&self) -> Result<Value, String> {
        let trimmed = self.search_value.trim();
        if trimmed.is_empty() {
            return Err("Empty value".to_string());
        }

        match self.value_type {
            ValueType::I8 => trimmed
                .parse::<i8>()
                .map(Value::I8)
                .map_err(|e| e.to_string()),
            ValueType::I16 => trimmed
                .parse::<i16>()
                .map(Value::I16)
                .map_err(|e| e.to_string()),
            ValueType::I32 => trimmed
                .parse::<i32>()
                .map(Value::I32)
                .map_err(|e| e.to_string()),
            ValueType::I64 => trimmed
                .parse::<i64>()
                .map(Value::I64)
                .map_err(|e| e.to_string()),
            ValueType::U8 => trimmed
                .parse::<u8>()
                .map(Value::U8)
                .map_err(|e| e.to_string()),
            ValueType::U16 => trimmed
                .parse::<u16>()
                .map(Value::U16)
                .map_err(|e| e.to_string()),
            ValueType::U32 => trimmed
                .parse::<u32>()
                .map(Value::U32)
                .map_err(|e| e.to_string()),
            ValueType::U64 => trimmed
                .parse::<u64>()
                .map(Value::U64)
                .map_err(|e| e.to_string()),
            ValueType::F32 => trimmed
                .parse::<f32>()
                .map(Value::F32)
                .map_err(|e| e.to_string()),
            ValueType::F64 => trimmed
                .parse::<f64>()
                .map(Value::F64)
                .map_err(|e| e.to_string()),
        }
    }

    pub(crate) fn parse_value_for_type(
        &self,
        value_str: &str,
        value_type: ValueType,
    ) -> Result<Value, String> {
        let trimmed = value_str.trim();
        if trimmed.is_empty() {
            return Err("Empty value".to_string());
        }

        match value_type {
            ValueType::I8 => trimmed
                .parse::<i8>()
                .map(Value::I8)
                .map_err(|e| e.to_string()),
            ValueType::I16 => trimmed
                .parse::<i16>()
                .map(Value::I16)
                .map_err(|e| e.to_string()),
            ValueType::I32 => trimmed
                .parse::<i32>()
                .map(Value::I32)
                .map_err(|e| e.to_string()),
            ValueType::I64 => trimmed
                .parse::<i64>()
                .map(Value::I64)
                .map_err(|e| e.to_string()),
            ValueType::U8 => trimmed
                .parse::<u8>()
                .map(Value::U8)
                .map_err(|e| e.to_string()),
            ValueType::U16 => trimmed
                .parse::<u16>()
                .map(Value::U16)
                .map_err(|e| e.to_string()),
            ValueType::U32 => trimmed
                .parse::<u32>()
                .map(Value::U32)
                .map_err(|e| e.to_string()),
            ValueType::U64 => trimmed
                .parse::<u64>()
                .map(Value::U64)
                .map_err(|e| e.to_string()),
            ValueType::F32 => trimmed
                .parse::<f32>()
                .map(Value::F32)
                .map_err(|e| e.to_string()),
            ValueType::F64 => trimmed
                .parse::<f64>()
                .map(Value::F64)
                .map_err(|e| e.to_string()),
        }
    }

    pub(crate) fn add_selected_to_list(&mut self) {
        for &index in &self.selected_result_indices {
            if index < self.scan_results.len() {
                let result = &self.scan_results[index];
                self.saved_addresses.push(SavedAddress {
                    address: result.address,
                    description: String::from("No description"),
                    value: result.value.clone(),
                    value_type: self.value_type,
                    frozen: false,
                });
            }
        }
        self.selected_result_indices.clear();
    }

    pub(crate) fn write_saved_value(&mut self, index: usize, new_value: Value) {
        if let Some(process) = &self.selected_process {
            if index < self.saved_addresses.len() {
                let address = self.saved_addresses[index].address;
                let reader = MemoryReader::new(process.pid);

                match reader.write_memory(address, &new_value.to_bytes()) {
                    Ok(_) => {
                        self.saved_addresses[index].value = new_value;
                        self.status_message = format!("Written to {:X}", address);
                    }
                    Err(e) => {
                        self.status_message = format!("Write error: {}", e);
                    }
                }
            }
        }
    }

    pub(crate) fn remove_selected_addresses(&mut self) {
        let mut indices = self.selected_saved_indices.clone();
        indices.sort_unstable_by(|a, b| b.cmp(a));
        indices.dedup();
        for index in indices {
            if index < self.saved_addresses.len() {
                self.saved_addresses.remove(index);
            }
        }
        self.selected_saved_indices.clear();
    }

    pub(crate) fn refresh_live_values(&mut self) {
        let process = match &self.selected_process {
            Some(p) => p,
            None => return,
        };

        if self.saved_addresses.is_empty() && self.scan_results.is_empty() {
            return;
        }

        let reader = MemoryReader::new(process.pid);

        for (i, saved) in self.saved_addresses.iter_mut().enumerate() {
            if self.editing_saved_value == Some(i) {
                continue;
            }
            let size = saved.value_type.size();
            if let Ok(data) = reader.read_value(saved.address, size) {
                if let Some(v) = Value::from_bytes(&data, saved.value_type) {
                    saved.value = v;
                }
            }
        }

        let max_refresh = self.scan_results.len().min(1000);
        let scan_size = self.scan_value_type.size();
        for found in self.scan_results.iter_mut().take(max_refresh) {
            if let Ok(data) = reader.read_value(found.address, scan_size) {
                if let Some(v) = Value::from_bytes(&data, self.scan_value_type) {
                    found.value = v;
                }
            }
        }
    }

    pub(crate) fn new_scan(&mut self) {
        self.scan_results.clear();
        self.selected_result_indices.clear();
        self.first_scan = true;
        self.is_scanning = false;
        self.scan_stream_receiver = None;
        self.scan_done_receiver = None;
        self.scan_progress_receiver = None;
        self.scan_progress_current = 0;
        self.scan_progress_total = 0;
        self.status_message = "Ready".to_string();
    }

    pub(crate) fn scan_type_needs_value(&self) -> bool {
        matches!(
            self.scan_type,
            ScanType::ExactValue | ScanType::GreaterThan | ScanType::LessThan
        )
    }

    pub(crate) fn render_scan_type_picker(&mut self, ui: &mut egui::Ui) {
        egui::ComboBox::from_id_salt("st")
            .selected_text(format!("{}", self.scan_type))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut self.scan_type, ScanType::ExactValue, "Exact Value");
                ui.selectable_value(&mut self.scan_type, ScanType::GreaterThan, "Bigger than...");
                ui.selectable_value(&mut self.scan_type, ScanType::LessThan, "Smaller than...");
                ui.selectable_value(
                    &mut self.scan_type,
                    ScanType::UnknownInitial,
                    "Unknown initial value",
                );

                if !self.first_scan {
                    ui.separator();
                    ui.selectable_value(
                        &mut self.scan_type,
                        ScanType::IncreasedValue,
                        "Increased value",
                    );
                    ui.selectable_value(
                        &mut self.scan_type,
                        ScanType::DecreasedValue,
                        "Decreased value",
                    );
                    ui.selectable_value(
                        &mut self.scan_type,
                        ScanType::ChangedValue,
                        "Changed value",
                    );
                    ui.selectable_value(
                        &mut self.scan_type,
                        ScanType::UnchangedValue,
                        "Unchanged value",
                    );
                }
            });
    }

    pub(crate) fn render_value_type_picker(&mut self, ui: &mut egui::Ui) {
        egui::ComboBox::from_id_salt("vt")
            .selected_text(format!("{}", self.value_type))
            .show_ui(ui, |ui| {
                for vt in ValueType::all() {
                    ui.selectable_value(&mut self.value_type, *vt, format!("{}", vt));
                }
            });
    }

    fn drain_scan_stream(&mut self) {
        let mut appended = false;

        if let Some(rx) = &self.scan_stream_receiver {
            for mut batch in rx.try_iter() {
                if batch.is_empty() {
                    continue;
                }
                self.scan_results.append(&mut batch);
                appended = true;
            }
        }

        if let Some(rx) = &self.scan_progress_receiver {
            for (current, total) in rx.try_iter() {
                self.scan_progress_current = current;
                self.scan_progress_total = total;
            }
        }

        if appended && self.is_scanning {
            self.status_message = format!("Found: {}", self.scan_results.len());
        }

        if let Some(done_rx) = &self.scan_done_receiver {
            if let Ok(result) = done_rx.try_recv() {
                match result {
                    Ok(()) => {
                        self.is_scanning = false;
                        self.first_scan = false;
                        self.scan_value_type = self.value_type;
                        if self.scan_progress_total > 0 {
                            self.scan_progress_current = self.scan_progress_total;
                        }
                        let count = self.scan_results.len();
                        self.status_message = if count == 0 {
                            "Found: 0 (hint: try Unknown initial value or check permissions)"
                                .to_string()
                        } else {
                            format!("Found: {}", count)
                        };
                    }
                    Err(msg) => {
                        if msg.contains("Permission denied") {
                            self.status_message = format!(
                                "{} (hint: run as root or allow ptrace via /proc/sys/kernel/yama/ptrace_scope)",
                                msg
                            );
                        } else {
                            self.status_message = msg;
                        }
                        self.is_scanning = false;
                    }
                }
                self.scan_done_receiver = None;
                self.scan_stream_receiver = None;
                self.scan_progress_receiver = None;
            }
        }
    }

    pub(crate) fn render_found_addresses(&mut self, ui: &mut egui::Ui) {
        self.drain_scan_stream();
        ui.horizontal(|ui| {
            ui.strong("Found addresses");
            ui.label(format!("Found: {}", self.scan_results.len()));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let can_add = !self.selected_result_indices.is_empty();
                if ui
                    .add_enabled(can_add, egui::Button::new("Add selected address"))
                    .clicked()
                {
                    self.add_selected_to_list();
                }
            });
        });
        ui.separator();

        let max_show = self.scan_results.len().min(10000);
        ui.push_id("found_addresses_table", |ui| {
            let mut found_table = FoundAddressesTable::new(
                &self.scan_results[..max_show],
                &mut self.selected_result_indices,
            );
            let mut table = found_table.table(egui::Id::new(("table", "found_addresses")));
            if self.scroll_to_top {
                self.scroll_to_top = false;
                table = table.scroll_to_row(0, None);
            }
            table.show(ui, &mut found_table);
            if let Some(row) = found_table.take_double_clicked_row() {
                self.selected_result_indices.clear();
                self.selected_result_indices.push(row);
                self.add_selected_to_list();
            }
        });

        if self.scan_results.len() > 10000 {
            ui.label("... (showing first 10000)");
        }

        // add button moved to header
    }

    pub(crate) fn render_saved_addresses(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.strong("Address list");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Remove").clicked() {
                    self.remove_selected_addresses();
                }
            });
        });
        ui.separator();

        let mut table_rows: Vec<TableSavedAddress> = self
            .saved_addresses
            .iter()
            .cloned()
            .map(TableSavedAddress::from)
            .collect();

        let value_updates = ui
            .push_id("saved_addresses_table", |ui| {
                let mut saved_table = SavedAddressesTable::new(
                    &mut table_rows,
                    &mut self.selected_saved_indices,
                    &mut self.value_edit_buffers,
                    &mut self.editing_saved_value,
                );
                let table = saved_table.table(egui::Id::new(("table", "saved_addresses")));
                table.show(ui, &mut saved_table);
                saved_table.take_updates()
            })
            .inner;

        self.saved_addresses = table_rows.into_iter().map(SavedAddress::from).collect();

        for (idx, vstr, vtype) in value_updates {
            match self.parse_value_for_type(&vstr, vtype) {
                Ok(nv) => self.write_saved_value(idx, nv),
                Err(e) => self.status_message = format!("Error: {}", e),
            }
        }
    }
}

impl From<SavedAddress> for TableSavedAddress {
    fn from(value: SavedAddress) -> Self {
        Self {
            address: value.address,
            description: value.description,
            value: value.value,
            value_type: value.value_type,
            frozen: value.frozen,
        }
    }
}

impl From<TableSavedAddress> for SavedAddress {
    fn from(value: TableSavedAddress) -> Self {
        Self {
            address: value.address,
            description: value.description,
            value: value.value,
            value_type: value.value_type,
            frozen: value.frozen,
        }
    }
}
