// Task: split app.rs into a slimmer UI entry that uses state/layout/tables modules.
use eframe::egui;

use super::layout::Layout;
use super::state::CheatEngineAppState;

pub struct CheatEngineApp {
    state: CheatEngineAppState,
}

impl Default for CheatEngineApp {
    fn default() -> Self {
        Self {
            state: CheatEngineAppState::default(),
        }
    }
}

impl eframe::App for CheatEngineApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.state.apply_ce_style(ctx);
        self.state.editing_saved_value = None;

        // Streaming scan updates are handled during table rendering.

        if self.state.show_process_list {
            let state = &mut self.state;
            egui::Window::new("Process List")
                .default_width(500.0)
                .default_height(400.0)
                .show(ctx, |ui| {
                    let filter = state.process_filter.trim().to_lowercase();

                    ui.push_id("process_list_scroll", |ui| {
                        egui::ScrollArea::vertical()
                            .max_height(10.0 * 24.0)
                            .show(ui, |ui| {
                                for process in &state.processes {
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
                                        state.selected_process = Some(process.clone());
                                        state.show_process_list = false;
                                        state.scan_results.clear();
                                        state.first_scan = true;
                                        state.status_message =
                                            format!("Process: {} ({})", process.name, process.pid);
                                    }
                                }
                            });
                    });

                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.label("Filter");
                        ui.add(
                            egui::TextEdit::singleline(&mut state.process_filter)
                                .desired_width(ui.available_width()),
                        );
                    });
                });
        }

        Layout::render(ctx, &mut self.state);

        if self.state.last_refresh.elapsed() >= self.state.refresh_interval {
            self.state.refresh_live_values();
            self.state.last_refresh = std::time::Instant::now();
        }

        if self.state.is_scanning {
            ctx.request_repaint();
        }
    }
}
