use std::sync::Arc;

use egui::mutex::RwLock;
use nessie::emu;

struct App {
    emu: Arc<RwLock<emu::Emu>>,
}

impl App {
    fn new() -> Self {
        Self {
            emu: Arc::new(RwLock::new(emu::Emu::new())),
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label("test");
            let r = self.emu.read();
            ui.label(format!("{}", (*r).cpu.a))
        });
    }
}

fn main() -> eframe::Result {
    env_logger::init();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1280.0, 720.0]),
        ..Default::default()
    };
    eframe::run_native("Nessie", options, Box::new(|_cc| Ok(Box::new(App::new()))))
}
