use eframe::egui;

use super::state::CheatEngineAppState;

/// Layout helpers for rendering the main UI sections.
/// This module focuses on arranging panels and delegating to per-section renderers.
pub struct Layout;

impl Layout {
    pub fn render(ctx: &egui::Context, state: &mut CheatEngineAppState) {
        Self::render_menu_bar(ctx, state);
        Self::render_status_bar(ctx, state);
        Self::render_scan_panel(ctx, state);
        Self::render_center(ctx, state);
    }

    fn render_menu_bar(ctx: &egui::Context, state: &mut CheatEngineAppState) {
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Open process...").clicked() {
                        state.open_process_list();
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

            ui.separator();

            ui.horizontal(|ui| {
                if ui.button("Select process").clicked() {
                    state.open_process_list();
                }
                if let Some(p) = &state.selected_process {
                    ui.label(format!("{} ({})", p.name, p.pid));
                } else {
                    ui.label("No Process Selected");
                }
            });
        });
    }

    fn render_status_bar(ctx: &egui::Context, state: &mut CheatEngineAppState) {
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(&state.status_message);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if state.is_scanning {
                        ui.label(format!("Scanning... found {}", state.scan_results.len()));
                    }
                });
            });
        });
    }

    fn render_scan_panel(ctx: &egui::Context, state: &mut CheatEngineAppState) {
        egui::SidePanel::right("scan_panel")
            .resizable(false)
            .min_width(280.0)
            .max_width(340.0)
            .show(ctx, |ui| {
                ui.heading("Scan");
                ui.separator();

                ui.horizontal(|ui| {
                    let can_first_scan = !state.is_scanning && state.first_scan;
                    let can_next_scan = !state.is_scanning && !state.scan_results.is_empty();

                    if state.first_scan {
                        if ui
                            .add_enabled(can_first_scan, egui::Button::new("First Scan"))
                            .clicked()
                        {
                            state.start_scan(ctx);
                        }
                    } else if ui.button("New Scan").clicked() {
                        state.new_scan();
                    }

                    if ui
                        .add_enabled(can_next_scan, egui::Button::new("Next Scan"))
                        .clicked()
                    {
                        state.start_scan(ctx);
                    }
                });

                ui.add_space(8.0);

                ui.group(|ui| {
                    ui.label("Value");
                    let needs_value = state.scan_type_needs_value();
                    ui.add_enabled(
                        needs_value,
                        egui::TextEdit::singleline(&mut state.search_value)
                            .desired_width(ui.available_width()),
                    );
                });

                ui.add_space(6.0);

                ui.group(|ui| {
                    ui.label("Scan type");
                    state.render_scan_type_picker(ui);
                });

                ui.add_space(6.0);

                ui.group(|ui| {
                    ui.label("Value type");
                    state.render_value_type_picker(ui);
                });
            });
    }

    fn render_center(ctx: &egui::Context, state: &mut CheatEngineAppState) {
        egui::CentralPanel::default().show(ctx, |ui| {
            if state.is_scanning && state.scan_progress_total > 0 {
                let progress =
                    state.scan_progress_current as f32 / state.scan_progress_total as f32;
                ui.add(egui::ProgressBar::new(progress).show_percentage());
                ui.separator();
            }

            let total_height = ui.available_height();
            let full_width = ui.available_width();
            let separator_height = ui.spacing().item_spacing.y + 2.0;
            let found_height = (total_height * 0.45).max(180.0);
            let list_height = (total_height - found_height - separator_height).max(140.0);

            ui.allocate_ui(egui::Vec2::new(full_width, found_height), |ui| {
                ui.set_min_height(found_height);
                ui.set_max_height(found_height);
                ui.set_width(full_width);
                state.render_found_addresses(ui);
            });

            ui.separator();

            ui.allocate_ui(egui::Vec2::new(full_width, list_height), |ui| {
                ui.set_min_height(list_height);
                ui.set_max_height(list_height);
                ui.set_width(full_width);
                state.render_saved_addresses(ui);
            });
        });
    }
}
