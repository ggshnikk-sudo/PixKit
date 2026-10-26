//! PixKit GUI — запускается без аргументов.
//! На Windows release-сборка идёт без консольного окна (wink), чтобы казуалам было проще.

#![cfg_attr(all(target_os = "windows", not(debug_assertions)), windows_subsystem = "windows")]

use eframe::egui;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([430.0, 470.0])
            .with_resizable(false)
            .with_title("PixKit"),
        centered: true,
        ..Default::default()
    };
    eframe::run_native(
        "PixKit",
        options,
        Box::new(|cc| Ok(Box::new(pixkit::gui::PixApp::new(cc)))),
    )
}