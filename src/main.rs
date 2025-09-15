use nessie::app::App;

fn main() -> eframe::Result {
    env_logger::init();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1280.0, 720.0]),
        ..Default::default()
    };
    eframe::run_native("Nessie", options, Box::new(|_cc| Ok(Box::new(App::new()))))
}
