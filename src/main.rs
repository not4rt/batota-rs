mod core;
mod ui;

use eframe::egui;
use ui::CheatEngineApp;

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
