use std::path::PathBuf;

use crate::{args::Args, platform::FileDataSource, ui::Ui};

pub struct App {
    ui: Ui,
}

impl App {
    pub fn new(ctx: &egui::Context, args: Args) -> Self {
        let mut ui = Ui::new(ctx);
        if let Some(rom) = &args.rom {
            ui.start(FileDataSource::Path(PathBuf::from(rom)));
        }

        Self { ui }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        self.ui.handle_input(ui.ctx());
        self.ui.handle_emu_events(ui.ctx(), frame);
        self.ui.draw(ui, frame);
    }
}
