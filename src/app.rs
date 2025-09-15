use std::{
    sync::{Arc, mpsc},
    thread,
};

use crate::{
    debug::DebugState,
    emu::{Command, Event, emu_thread},
};

pub struct App {
    command_tx: mpsc::Sender<Command>,
    event_rx: mpsc::Receiver<Event>,

    debug_state: Arc<DebugState>,
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
            debug_state,
        }
    }

    fn send_command(&self, command: Command) {
        if let Err(e) = self.command_tx.send(command) {}
    }

    fn handle_events(&mut self) {
        while let Ok(event) = self.event_rx.try_recv() {
            match event {
                Event::Started => {}
                Event::Stopped => {}
            }
        }
    }

    fn ui(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("menubar").show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Quit").clicked() {
                        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
                ui.menu_button("Emulator", |ui| {
                    if ui.button("Start").clicked() {
                        self.send_command(Command::Start);
                    }
                    if ui.button("Stop").clicked() {
                        self.send_command(Command::Stop);
                    }
                });
            });
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label(format!("{}", self.debug_state.cpu.read().unwrap().a))
        });
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.handle_events();
        self.ui(ctx, frame);
    }
}
