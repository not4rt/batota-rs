use egui::{Id, Margin, Ui};

use crate::core::{FoundAddress, Value, ValueType};

#[derive(Clone)]
pub struct SavedAddress {
    pub address: usize,
    pub description: String,
    pub value: Value,
    pub value_type: ValueType,
    pub frozen: bool,
}

pub struct FoundAddressesTable<'a> {
    rows: &'a [FoundAddress],
    selected_indices: &'a mut Vec<usize>,
    double_clicked_row: Option<usize>,
    prefetched: Vec<egui_table::PrefetchInfo>,
}

impl<'a> FoundAddressesTable<'a> {
    pub fn new(rows: &'a [FoundAddress], selected_indices: &'a mut Vec<usize>) -> Self {
        Self {
            rows,
            selected_indices,
            double_clicked_row: None,
            prefetched: vec![],
        }
    }

    pub fn take_double_clicked_row(&mut self) -> Option<usize> {
        self.double_clicked_row.take()
    }

    pub fn table(&self, id_salt: Id) -> egui_table::Table {
        egui_table::Table::new()
            .id_salt(id_salt)
            .num_rows(self.rows.len() as u64)
            .columns(vec![
                egui_table::Column::new(140.0)
                    .range(80.0..=260.0)
                    .resizable(true),
                egui_table::Column::new(100.0)
                    .range(60.0..=260.0)
                    .resizable(true),
            ])
            .headers([egui_table::HeaderRow::new(22.0)])
    }
}

impl<'a> egui_table::TableDelegate for FoundAddressesTable<'a> {
    fn prepare(&mut self, info: &egui_table::PrefetchInfo) {
        self.prefetched.push(info.clone());
    }

    fn header_cell_ui(&mut self, ui: &mut egui::Ui, cell_info: &egui_table::HeaderCellInfo) {
        let col = cell_info.col_range.start;
        let label = match col {
            0 => "Address",
            1 => "Value",
            _ => "",
        };

        egui::Frame::NONE
            .inner_margin(Margin::symmetric(4, 0))
            .show(ui, |ui| {
                ui.strong(label);
            });
    }

    fn row_ui(&mut self, ui: &mut Ui, _row_nr: u64) {
        if ui.rect_contains_pointer(ui.max_rect()) {
            ui.painter()
                .rect_filled(ui.max_rect(), 0.0, ui.visuals().code_bg_color);
        }
    }

    fn cell_ui(&mut self, ui: &mut egui::Ui, cell_info: &egui_table::CellInfo) {
        let row = cell_info.row_nr as usize;
        if row >= self.rows.len() {
            return;
        }
        let data = &self.rows[row];

        egui::Frame::NONE
            .inner_margin(Margin::symmetric(4, 0))
            .show(ui, |ui| match cell_info.col_nr {
                0 => {
                    let selected = self.selected_indices.contains(&row);
                    let resp = ui.selectable_label(selected, format!("{:016X}", data.address));
                    if resp.clicked() {
                        if ui.input(|i| i.modifiers.ctrl) {
                            if selected {
                                self.selected_indices.retain(|&x| x != row);
                            } else if !self.selected_indices.contains(&row) {
                                self.selected_indices.push(row);
                            }
                        } else {
                            self.selected_indices.clear();
                            self.selected_indices.push(row);
                        }
                    }
                    if resp.double_clicked() {
                        self.double_clicked_row = Some(row);
                    }
                }
                1 => {
                    ui.label(format!("{}", data.value));
                }
                _ => {}
            });
    }
}

pub struct SavedAddressesTable<'a> {
    rows: &'a mut [SavedAddress],
    selected_indices: &'a mut Vec<usize>,
    value_edit_buffers: &'a mut Vec<String>,
    editing_saved_value: &'a mut Option<usize>,
    pending_updates: Vec<(usize, String, ValueType)>,
    pending_changes: Option<(usize, String, ValueType)>, // (row, value_str, value_type)
    prefetched: Vec<egui_table::PrefetchInfo>,
}

impl<'a> SavedAddressesTable<'a> {
    pub fn new(
        rows: &'a mut [SavedAddress],
        selected_indices: &'a mut Vec<usize>,
        value_edit_buffers: &'a mut Vec<String>,
        editing_saved_value: &'a mut Option<usize>,
    ) -> Self {
        if value_edit_buffers.len() != rows.len() {
            value_edit_buffers.resize(rows.len(), String::new());
        }

        Self {
            rows,
            selected_indices,
            value_edit_buffers,
            editing_saved_value,
            pending_updates: vec![],
            pending_changes: None,
            prefetched: vec![],
        }
    }

    pub fn table(&self, id_salt: Id) -> egui_table::Table {
        egui_table::Table::new()
            .id_salt(id_salt)
            .num_rows(self.rows.len() as u64)
            .columns(vec![
                egui_table::Column::new(24.0).range(20.0..=40.0),
                egui_table::Column::new(120.0)
                    .range(80.0..=220.0)
                    .resizable(true),
                egui_table::Column::new(140.0)
                    .range(80.0..=240.0)
                    .resizable(true),
                egui_table::Column::new(60.0)
                    .range(40.0..=90.0)
                    .resizable(true),
                egui_table::Column::new(100.0)
                    .range(60.0..=200.0)
                    .resizable(true),
            ])
            .headers([egui_table::HeaderRow::new(22.0)])
    }

    pub fn take_updates(&mut self) -> Vec<(usize, String, ValueType)> {
        std::mem::take(&mut self.pending_updates)
    }

    pub fn take_changed(&mut self) -> Option<(usize, String, ValueType)> {
        self.pending_changes.take()
    }
}

impl<'a> egui_table::TableDelegate for SavedAddressesTable<'a> {
    fn prepare(&mut self, info: &egui_table::PrefetchInfo) {
        self.prefetched.push(info.clone());
    }

    fn header_cell_ui(&mut self, ui: &mut egui::Ui, cell_info: &egui_table::HeaderCellInfo) {
        let col = cell_info.col_range.start;
        let label = match col {
            0 => "X",
            1 => "Description",
            2 => "Address",
            3 => "Type",
            4 => "Value",
            _ => "",
        };

        egui::Frame::NONE
            .inner_margin(Margin::symmetric(4, 0))
            .show(ui, |ui| {
                ui.strong(label);
            });
    }

    fn row_ui(&mut self, ui: &mut Ui, _row_nr: u64) {
        if ui.rect_contains_pointer(ui.max_rect()) {
            ui.painter()
                .rect_filled(ui.max_rect(), 0.0, ui.visuals().code_bg_color);
        }
    }

    fn cell_ui(&mut self, ui: &mut egui::Ui, cell_info: &egui_table::CellInfo) {
        let row = cell_info.row_nr as usize;
        if row >= self.rows.len() {
            return;
        }
        let data = &mut self.rows[row];

        egui::Frame::NONE
            .inner_margin(Margin::symmetric(4, 0))
            .show(ui, |ui| match cell_info.col_nr {
                0 => {
                    let selected = self.selected_indices.contains(&row);
                    let cb = ui.checkbox(&mut data.frozen, "");
                    if cb.clicked() {
                        if selected {
                            self.selected_indices.retain(|&x| x != row);
                        } else if !self.selected_indices.contains(&row) {
                            self.selected_indices.push(row);
                        }
                    }
                }
                1 => {
                    ui.add(
                        egui::TextEdit::singleline(&mut data.description)
                            .desired_width(ui.available_width()),
                    );
                }
                2 => {
                    ui.label(format!("{:X}", data.address));
                }
                3 => {
                    ui.label(format!("{}", data.value_type));
                }
                4 => {
                    let edit_id = Id::new(("saved_value", row));
                    let is_editing = ui.memory(|mem| mem.has_focus(edit_id));
                    if !is_editing {
                        self.value_edit_buffers[row] = format!("{}", data.value);
                    }

                    let ve = ui.add(
                        egui::TextEdit::singleline(&mut self.value_edit_buffers[row])
                            .desired_width(ui.available_width())
                            .id(edit_id),
                    );
                    if ve.has_focus() {
                        *self.editing_saved_value = Some(row);
                    }
                    if ve.changed() {
                        // Track text changes for debounced write
                        let new_value_str = self.value_edit_buffers[row].clone();
                        self.pending_changes = Some((row, new_value_str, data.value_type));
                    }
                    if ve.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        // Immediate write on Enter key
                        let new_value_str = self.value_edit_buffers[row].clone();
                        self.pending_updates
                            .push((row, new_value_str, data.value_type));
                    }
                }
                _ => {}
            });
    }
}
