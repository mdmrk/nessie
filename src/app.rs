use crate::{args::Args, ui::Ui};

pub struct App {
    ui: Ui,
}

impl App {
    pub fn new(ctx: &egui::Context, args: Args) -> Self {
        let mut ui = Ui::new(ctx, args.clone());
        if let Some(rom) = &args.rom {
            ui.spawn_emu_thread(rom);
        }

        Self { ui }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        #[cfg(all(debug_assertions, not(target_arch = "wasm32")))]
        puffin::GlobalProfiler::lock().new_frame();
        self.ui.handle_input(ctx);
        self.ui.handle_emu_events(ctx, frame);
        self.ui.draw(ctx, frame);
    }
}
