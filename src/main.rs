mod board;
mod engine;
mod gui;
mod moves;
mod uci;

use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.iter().any(|a| a == "--uci") {
        uci::run_uci();
        return;
    }

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("StockShark")
            .with_inner_size([920.0, 720.0])
            .with_min_inner_size([700.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "StockShark",
        options,
        Box::new(|cc| Ok(Box::new(gui::StockSharkApp::new(cc)))),
    ).unwrap();
}
