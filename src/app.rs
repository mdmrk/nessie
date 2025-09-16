use std::{
    sync::{Arc, mpsc},
    thread,
};

use log::error;

use crate::{
    debug::DebugState,
    emu::{Command, Event, emu_thread},
    ui::Ui,
};

pub struct App {
    command_tx: mpsc::Sender<Command>,
    event_rx: mpsc::Receiver<Event>,

    ui: Ui,
}

impl App {
    pub fn new() -> Self {
        let (command_tx, command_rx) = mpsc::channel();
        let (event_tx, event_rx) = mpsc::channel();

        let debug_state = Arc::new(DebugState::new());

        let debug_clone = debug_state.clone();
        thread::spawn(move || {
            emu_thread(command_rx, event_tx, debug_clone);
        });

        Self {
            command_tx,
            event_rx,
            ui: Ui::new(debug_state),
        }
    }

    pub fn send_command(&self, command: Command) {
        if let Err(e) = self.command_tx.send(command) {
            error!("{e}");
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
        self.ui.draw(self, ctx, frame);
    }
}
