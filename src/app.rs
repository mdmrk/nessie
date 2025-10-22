use std::sync::Arc;

use egui::{Key, KeyboardShortcut};

use crate::{args::Args, debug::DebugState, ui::Ui};

struct Shortcut {
    name: &'static str,
    keyboard_shortcut: KeyboardShortcut,
}

pub struct App {
    ui: Ui,
}

impl App {
    pub fn new(args: &Args) -> Self {
        let debug_state = Arc::new(DebugState::new());

        let mut ui = Ui::new(debug_state, args.clone());
        if let Some(rom) = &args.rom {
            ui.spawn_emu_thread(rom);
            if args.pause {
                ui.emu_pause();
            }
        }

        Self { ui }
    }

    fn listen_shortcuts(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let shortcuts: &[Shortcut] = &[
            Shortcut {
                name: "step",
                keyboard_shortcut: KeyboardShortcut {
                    modifiers: Default::default(),
                    logical_key: Key::Enter,
                },
            },
            Shortcut {
                name: "pauseresume",
                keyboard_shortcut: KeyboardShortcut {
                    modifiers: Default::default(),
                    logical_key: Key::Space,
                },
            },
        ];

        ctx.input_mut(|i| {
            for shortcut in shortcuts {
                if i.consume_shortcut(&shortcut.keyboard_shortcut) && shortcut.name == "step" {
                    if self.ui.is_paused() {
                        self.ui.emu_step();
                    }
                }
                if i.consume_shortcut(&shortcut.keyboard_shortcut) && shortcut.name == "pauseresume"
                {
                    if self.ui.is_paused() {
                        self.ui.emu_resume()
                    } else {
                        self.ui.emu_pause();
                    }
                }
            }
        });
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.listen_shortcuts(ctx, frame);
        self.ui.handle_emu_events(ctx, frame);
        self.ui.draw(ctx, frame);
    }
}
