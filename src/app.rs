use std::{
    sync::{Arc, mpsc},
    thread,
};

use crate::{
    args::Args,
    debug::DebugState,
    emu::{Event, emu_thread},
    ui::Ui,
};

pub struct App {
    event_rx: mpsc::Receiver<Event>,

    ui: Ui,
}

impl App {
    pub fn new(args: &Args) -> Self {
        let (command_tx, command_rx) = mpsc::channel();
        let (event_tx, event_rx) = mpsc::channel();

        let debug_state = Arc::new(DebugState::new());

        let debug_clone = debug_state.clone();
        let args_clone = args.clone();
        thread::spawn(move || {
            emu_thread(command_rx, event_tx, debug_clone, &args_clone);
        });

        Self {
            event_rx,
            ui: Ui::new(command_tx, debug_state),
        }
    }

    pub fn handle_events(&mut self) {
        while let Ok(event) = self.event_rx.try_recv() {
            match event {
                Event::Started => {}
                Event::Stopped => {}
            }
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.handle_events();
        self.ui.draw(ctx, frame);
    }
}
