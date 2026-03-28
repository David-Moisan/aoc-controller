#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod monitor;
mod app;

use app::MonitorApp;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Monitor Control")
            .with_inner_size([480.0, 420.0])   // compact window
            .with_min_inner_size([380.0, 320.0]), // resizable but not tiny
        ..Default::default()
    };

    eframe::run_native(
        "Monitor Control",
        options,
        // Box::new wraps our app in a heap allocation — eframe requires this
        Box::new(|cc| Box::new(MonitorApp::new(cc))),
    )
}