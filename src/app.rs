use std::{
    sync::{Arc, mpsc},
    thread,
};

use egui::{Key, KeyboardShortcut};

use crate::{
    args::Args,
    debug::DebugState,
    emu::{Command, emu_thread},
    ui::Ui,
};

pub struct App {
    ui: Ui,
}

impl App {
    pub fn new(args: &Args) -> Self {
        let (command_tx, command_rx) = mpsc::channel();

        let debug_state = Arc::new(DebugState::new());

        let debug_clone = debug_state.clone();
        let args_clone = args.clone();
        thread::spawn(move || {
            emu_thread(command_rx, debug_clone, &args_clone);
        });

        let mut ui = Ui::new(command_tx, debug_state);
        if args.pause {
            ui.send_command(Command::Pause);
            ui.paused = true;
        }

        Self { ui }
    }

    fn listen_shortcuts(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let shortcuts: &[KeyboardShortcut] = &[
            // Step
            KeyboardShortcut {
                modifiers: Default::default(),
                logical_key: Key::ArrowDown,
            },
        ];

        ctx.input(|i| {
            for shortcut in shortcuts {
                if i.consume_shortcut(shortcut) {}
            }
        });
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.listen_shortcuts(ctx, frame);
        self.ui.draw(ctx, frame);
    }
}
