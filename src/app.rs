use std::{sync::mpsc, thread};

use crate::emu::{Command, Event, emu_thread};

pub struct App {
    command_tx: mpsc::Sender<Command>,
    event_rx: mpsc::Receiver<Event>,
}

impl App {
    pub fn new() -> Self {
        let (command_tx, command_rx) = mpsc::channel();
        let (event_tx, event_rx) = mpsc::channel();

        thread::spawn(move || {
            emu_thread(command_rx, event_tx);
        });

        Self {
            command_tx,
            event_rx,
        }
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
        egui::CentralPanel::default().show(ctx, |ui| {});
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.handle_events();
        self.ui(ctx, frame);
    }
}
