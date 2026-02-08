use std::path::PathBuf;

use crate::{args::Args, platform::RomSource, ui::Ui};

pub struct App {
    ui: Ui,
}

impl App {
    pub fn new(ctx: &egui::Context, args: Args) -> Self {
        let mut ui = Ui::new(ctx, args.clone());
        if let Some(rom) = &args.rom {
            ui.start(RomSource::Path(PathBuf::from(rom)));
        }

        Self { ui }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.ui.handle_input(ctx);
        self.ui.handle_emu_events(ctx, frame);
        self.ui.draw(ctx, frame);
    }
}
