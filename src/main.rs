use clap::Parser;
use nessie::{app::App, args::Args};

fn main() -> eframe::Result {
    env_logger::init();

    let args = Args::parse();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1280.0, 720.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Nessie",
        options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(App::new(&args)))
        }),
    )
}
