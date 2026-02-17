mod memory;
mod process;
mod scanner;
mod types;

use eframe::egui;
use memory::MemoryReader;
use process::{list_processes, Process};
use scanner::Scanner;
use std::sync::mpsc::{channel, Receiver};
use std::thread;
use std::time::{Duration, Instant};
use types::{FoundAddress, ScanType, Value, ValueType};

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1000.0, 700.0])
            .with_min_inner_size([800.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Cheat Engine",
        options,
        Box::new(|_cc| Ok(Box::new(CheatEngineApp::default()))),
    )
}

struct CheatEngineApp {
    processes: Vec<Process>,
    selected_process: Option<Process>,
    show_process_list: bool,
    process_filter: String,

    value_type: ValueType,
    scan_type: ScanType,
    search_value: String,

    scan_results: Vec<FoundAddress>,
    saved_addresses: Vec<SavedAddress>,
    is_scanning: bool,
    scan_receiver: Option<Receiver<ScanResult>>,
    first_scan: bool,

    selected_result_indices: Vec<usize>,
    selected_saved_indices: Vec<usize>,
    status_message: String,
    scroll_to_top: bool,
    value_edit_buffers: Vec<String>,
    editing_saved_value: Option<usize>,
    scan_value_type: ValueType,
    last_refresh: Instant,
    refresh_interval: Duration,
}

enum ScanResult {
    Complete(Vec<FoundAddress>),
    Error(String),
}

#[derive(Clone)]
struct SavedAddress {
    address: usize,
    description: String,
    value: Value,
    value_type: ValueType,
    frozen: bool,
}

impl Default for CheatEngineApp {
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

impl CheatEngineApp {
    fn open_process_list(&mut self) {
        match list_processes() {
            Ok(procs) => {
                self.processes = procs;
                self.show_process_list = true;
            }
            Err(e) => self.status_message = format!("Error: {}", e),
        }
    }

    fn apply_ce_style(&self, ctx: &egui::Context) {
        let mut style = (*ctx.style()).clone();
        style.spacing.item_spacing = egui::vec2(6.0, 4.0);
        style.spacing.window_margin = egui::Margin::same(6);
        ctx.set_style(style);
    }

    fn start_scan(&mut self, ctx: &egui::Context) {
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

        let (tx, rx) = channel();
        self.scan_receiver = Some(rx);
        let ctx_clone = ctx.clone();

        let existing_results = if self.first_scan {
            Vec::new()
        } else {
            self.scan_results.clone()
        };

        let is_first_scan = self.first_scan;

        self.is_scanning = true;
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

            let results = if is_first_scan {
                scanner.initial_scan(scan_type, target_value)
            } else {
                scanner.next_scan(&existing_results, scan_type, target_value)
            };

            match results {
                Ok(addrs) => {
                    let _ = tx.send(ScanResult::Complete(addrs));
                }
                Err(e) => {
                    let _ = tx.send(ScanResult::Error(format!("{}", e)));
                }
            }
            ctx_clone.request_repaint();
        });
    }

    fn parse_value(&self) -> Result<Value, String> {
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

    fn add_selected_to_list(&mut self) {
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

    fn write_saved_value(&mut self, index: usize, new_value: Value) {
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

    fn parse_value_for_type(
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

    fn remove_selected_addresses(&mut self) {
        let mut indices = self.selected_saved_indices.clone();
        indices.sort_by(|a, b| b.cmp(a));
        for index in indices {
            if index < self.saved_addresses.len() {
                self.saved_addresses.remove(index);
            }
        }
        self.selected_saved_indices.clear();
    }

    fn refresh_live_values(&mut self) {
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
}

impl eframe::App for CheatEngineApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.apply_ce_style(ctx);
        self.editing_saved_value = None;

        if let Some(receiver) = &self.scan_receiver {
            if let Ok(result) = receiver.try_recv() {
                match result {
                    ScanResult::Complete(results) => {
                        self.scan_results = results;
                        self.is_scanning = false;
                        self.first_scan = false;
                        self.scan_value_type = self.value_type;
                        let count = self.scan_results.len();
                        self.status_message = if count == 0 {
                            "Found: 0 (hint: try Unknown initial value or check permissions)"
                                .to_string()
                        } else {
                            format!("Found: {}", count)
                        };
                        self.scan_receiver = None;
                    }
                    ScanResult::Error(msg) => {
                        if msg.contains("Permission denied") {
                            self.status_message = format!(
                                "{} (hint: run as root or allow ptrace via /proc/sys/kernel/yama/ptrace_scope)",
                                msg
                            );
                        } else {
                            self.status_message = msg;
                        }
                        self.is_scanning = false;
                        self.scan_receiver = None;
                    }
                }
            }
        }

        if self.show_process_list {
            egui::Window::new("Process List")
                .default_width(500.0)
                .default_height(400.0)
                .show(ctx, |ui| {
                    let filter = self.process_filter.trim().to_lowercase();

                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for process in &self.processes {
                            if !filter.is_empty()
                                && !process.name.to_lowercase().contains(&filter)
                                && !process.pid.to_string().contains(&filter)
                            {
                                continue;
                            }

                            if ui
                                .button(format!("{} - {}", process.name, process.pid))
                                .clicked()
                            {
                                self.selected_process = Some(process.clone());
                                self.show_process_list = false;
                                self.scan_results.clear();
                                self.first_scan = true;
                                self.status_message =
                                    format!("Process: {} ({})", process.name, process.pid);
                            }
                        }
                    });

                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.label("Filter");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.process_filter)
                                .desired_width(ui.available_width()),
                        );
                    });
                });
        }

        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Open process...").clicked() {
                        self.open_process_list();
                        ui.close();
                    }
                });
                ui.menu_button("Edit", |ui| {
                    ui.label("Coming soon");
                });
                ui.menu_button("Help", |ui| {
                    ui.label("Cheat Engine (egui)");
                });
            });
        });

        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(&self.status_message);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if self.is_scanning {
                        ui.label("Scanning...");
                    }
                });
            });
        });

        egui::SidePanel::left("scan_panel")
            .resizable(false)
            .min_width(280.0)
            .max_width(340.0)
            .show(ctx, |ui| {
                ui.heading("Scan");
                ui.separator();

                ui.horizontal(|ui| {
                    if ui.button("Select process").clicked() {
                        self.open_process_list();
                    }
                    if let Some(p) = &self.selected_process {
                        ui.label(format!("{} ({})", p.name, p.pid));
                    }
                });

                ui.add_space(6.0);

                ui.group(|ui| {
                    ui.label("Value type");
                    egui::ComboBox::from_id_salt("vt")
                        .selected_text(format!("{}", self.value_type))
                        .show_ui(ui, |ui| {
                            for vt in ValueType::all() {
                                ui.selectable_value(&mut self.value_type, *vt, format!("{}", vt));
                            }
                        });
                });

                ui.add_space(6.0);

                ui.group(|ui| {
                    ui.label("Scan type");
                    egui::ComboBox::from_id_salt("st")
                        .selected_text(format!("{}", self.scan_type))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.scan_type,
                                ScanType::ExactValue,
                                "Exact Value",
                            );
                            ui.selectable_value(
                                &mut self.scan_type,
                                ScanType::GreaterThan,
                                "Bigger than...",
                            );
                            ui.selectable_value(
                                &mut self.scan_type,
                                ScanType::LessThan,
                                "Smaller than...",
                            );
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
                });

                ui.add_space(6.0);

                ui.group(|ui| {
                    ui.label("Value");
                    let needs_value = matches!(
                        self.scan_type,
                        ScanType::ExactValue | ScanType::GreaterThan | ScanType::LessThan
                    );
                    ui.add_enabled(
                        needs_value,
                        egui::TextEdit::singleline(&mut self.search_value)
                            .desired_width(ui.available_width()),
                    );
                });

                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    let scan_label = if self.first_scan {
                        "First Scan"
                    } else {
                        "Next Scan"
                    };
                    if ui
                        .add_enabled(!self.is_scanning, egui::Button::new(scan_label))
                        .clicked()
                    {
                        self.start_scan(ctx);
                    }
                    if ui.button("New Scan").clicked() {
                        self.scan_results.clear();
                        self.first_scan = true;
                        self.status_message = "Ready".to_string();
                    }
                });

                ui.add_space(8.0);
                ui.separator();

                if ui.button("Add selected address").clicked() {
                    self.add_selected_to_list();
                }
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.set_width(ui.available_width() * 0.5 - 6.0);
                    ui.set_min_height(ui.available_height());
                    ui.strong("Found addresses");
                    ui.label(format!("{}", self.scan_results.len()));
                    ui.separator();

                    ui.horizontal(|ui| {
                        ui.label("Address");
                        ui.add_space(80.0);
                        ui.label("Value");
                    });
                    ui.separator();
                    ui.add_space(2.0);

                    let avail_height = ui.available_height();
                    let scroll = if self.scroll_to_top {
                        self.scroll_to_top = false;
                        egui::ScrollArea::vertical()
                            .id_salt("found_addresses_scroll")
                            .max_height(avail_height)
                            .auto_shrink([false, false])
                            .scroll_offset(egui::Vec2::ZERO)
                    } else {
                        egui::ScrollArea::vertical()
                            .id_salt("found_addresses_scroll")
                            .max_height(avail_height)
                            .auto_shrink([false, false])
                    };

                    scroll.show(ui, |ui| {
                        ui.set_min_height(avail_height);
                        ui.set_min_width(ui.available_width());
                        let max_show = self.scan_results.len().min(10000);

                        for (i, res) in self.scan_results.iter().take(max_show).enumerate() {
                            let selected = self.selected_result_indices.contains(&i);

                            ui.horizontal(|ui| {
                                let r =
                                    ui.selectable_label(selected, format!("{:016X}", res.address));
                                if r.clicked() {
                                    if ui.input(|i| i.modifiers.ctrl) {
                                        if selected {
                                            self.selected_result_indices.retain(|&x| x != i);
                                        } else {
                                            self.selected_result_indices.push(i);
                                        }
                                    } else {
                                        self.selected_result_indices.clear();
                                        self.selected_result_indices.push(i);
                                    }
                                }
                                ui.add_space(10.0);
                                ui.label(format!("{}", res.value));
                            });
                        }

                        if self.scan_results.len() > 10000 {
                            ui.label("... (showing first 10000)");
                        }
                    });
                });

                ui.separator();

                ui.vertical(|ui| {
                    ui.set_width(ui.available_width());
                    ui.set_min_height(ui.available_height());
                    ui.strong("Address list");
                    ui.separator();

                    if ui.button("Remove").clicked() {
                        self.remove_selected_addresses();
                    }

                    ui.separator();

                    ui.horizontal(|ui| {
                        ui.label("X");
                        ui.label("Description");
                        ui.add_space(20.0);
                        ui.label("Address");
                        ui.add_space(20.0);
                        ui.label("Type");
                        ui.add_space(10.0);
                        ui.label("Value");
                    });
                    ui.separator();
                    ui.add_space(2.0);

                    let avail_height = ui.available_height();
                    egui::ScrollArea::vertical()
                        .id_salt("address_list_scroll")
                        .max_height(avail_height)
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.set_min_height(avail_height);
                            ui.set_min_width(ui.available_width());
                            let mut value_updates = Vec::new();

                            if self.value_edit_buffers.len() != self.saved_addresses.len() {
                                self.value_edit_buffers
                                    .resize(self.saved_addresses.len(), String::new());
                            }

                            for (i, saved) in self.saved_addresses.iter_mut().enumerate() {
                                let selected = self.selected_saved_indices.contains(&i);
                                let edit_id = egui::Id::new(("saved_value", i));
                                let is_editing = ui.memory(|mem| mem.has_focus(edit_id));
                                if !is_editing {
                                    self.value_edit_buffers[i] = format!("{}", saved.value);
                                }

                                ui.horizontal(|ui| {
                                    let cb = ui.checkbox(&mut saved.frozen, "");
                                    if cb.clicked() {
                                        if !ui.input(|i| i.modifiers.ctrl) {
                                            self.selected_saved_indices.clear();
                                        }
                                        if selected {
                                            self.selected_saved_indices.retain(|&x| x != i);
                                        } else {
                                            self.selected_saved_indices.push(i);
                                        }
                                    }

                                    ui.add(
                                        egui::TextEdit::singleline(&mut saved.description)
                                            .desired_width(100.0),
                                    );
                                    ui.label(format!("{:X}", saved.address));
                                    ui.label(format!("{}", saved.value_type));

                                    let ve = ui.add(
                                        egui::TextEdit::singleline(&mut self.value_edit_buffers[i])
                                            .desired_width(80.0)
                                            .id(edit_id),
                                    );
                                    if ve.has_focus() {
                                        self.editing_saved_value = Some(i);
                                    }
                                    if ve.lost_focus()
                                        && ui.input(|i| i.key_pressed(egui::Key::Enter))
                                    {
                                        let new_value_str = self.value_edit_buffers[i].clone();
                                        value_updates.push((i, new_value_str, saved.value_type));
                                    }
                                });
                            }

                            for (idx, vstr, vtype) in value_updates {
                                match self.parse_value_for_type(&vstr, vtype) {
                                    Ok(nv) => self.write_saved_value(idx, nv),
                                    Err(e) => self.status_message = format!("Error: {}", e),
                                }
                            }
                        });
                });
            });
        });

        if self.last_refresh.elapsed() >= self.refresh_interval {
            self.refresh_live_values();
            self.last_refresh = Instant::now();
        }

        if self.is_scanning {
            ctx.request_repaint();
        }
    }
}
