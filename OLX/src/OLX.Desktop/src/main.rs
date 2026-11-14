mod models;
mod api;
mod app;
mod views;

use app::OlxApp;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1024.0, 768.0])
            .with_title("OLX - Buy & Sell"),
        ..Default::default()
    };
    eframe::run_native("OLX Desktop", options, Box::new(|cc| Box::new(OlxApp::new(cc))))
}
