use bytesize::ByteSize;
use log::error;
use std::sync::{Arc, mpsc};

use crate::{cpu::Flags, debug::DebugState, emu::Command};

pub struct Ui {
    command_tx: mpsc::Sender<Command>,

    debug_state: Arc<DebugState>,

    mem_search: String,

    running: bool,
    paused: bool,
}

impl Ui {
    pub fn new(command_tx: mpsc::Sender<Command>, debug_state: Arc<DebugState>) -> Self {
        Self {
            command_tx,
            debug_state,
            mem_search: "".into(),
            running: true,
            paused: false,
        }
    }

    pub fn send_command(&self, command: Command) {
        if let Err(e) = self.command_tx.send(command) {
            error!("{e}");
        }
    }

    pub fn emu_step(&mut self) {
        self.send_command(Command::Step);
    }

    pub fn emu_resume(&mut self) {
        self.paused = false;
        self.send_command(Command::Resume);
    }

    pub fn emu_pause(&mut self) {
        self.paused = true;
        self.send_command(Command::Pause);
    }

    pub fn emu_stop(&mut self) {
        // TODO: define stop ??
        self.send_command(Command::Stop);
        self.running = false;
    }

    pub fn draw(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("menubar")
            .resizable(false)
            .show(ctx, |ui| {
                egui::MenuBar::new().ui(ui, |ui| {
                    ui.menu_button("File", |ui| {
                        if ui.button("Quit").clicked() {
                            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });
                    ui.menu_button("Emulator", |ui| {
                        ui.add_enabled_ui(self.paused, |ui| {
                            if ui.button("Step").clicked() {
                                self.emu_step();
                            }
                        });
                        ui.add_enabled_ui(self.paused, |ui| {
                            if ui.button("Resume").clicked() {
                                self.emu_resume();
                            }
                        });
                        ui.add_enabled_ui(!self.paused, |ui| {
                            if ui.button("Pause").clicked() {
                                self.emu_pause();
                            }
                        });
                        ui.add_enabled_ui(self.running, |ui| {
                            if ui.button("Stop").clicked() {
                                self.emu_stop();
                            }
                        });
                    });
                });
            });
        egui::SidePanel::left("left_panel")
            .resizable(true)
            .default_width(200.0)
            .show(ctx, |ui| {
                if let Ok(cpu) = self.debug_state.cpu.read() {
                    ui.heading("CPU");
                    egui::Grid::new("cpu_grid")
                        .num_columns(3)
                        .striped(true)
                        .show(ui, |ui| {
                            ui.add(egui::Label::new("sp"));
                            ui.label(format!("{}", cpu.sp + 0x100));
                            ui.label(format!("0x{:04X}", cpu.sp + 0x100));
                            ui.end_row();
                            ui.add(egui::Label::new("pc"));
                            ui.label(format!("{}", cpu.pc));
                            ui.label(format!("0x{:04X}", cpu.pc));
                            ui.end_row();
                            ui.add(egui::Label::new("a"));
                            ui.label(format!("{}", cpu.a));
                            ui.label(format!("0x{:02X}", cpu.a));
                            ui.end_row();
                            ui.add(egui::Label::new("x"));
                            ui.label(format!("{}", cpu.x));
                            ui.label(format!("0x{:02X}", cpu.x));
                            ui.end_row();
                            ui.add(egui::Label::new("y"));
                            ui.label(format!("{}", cpu.y));
                            ui.label(format!("0x{:02X}", cpu.y));
                            ui.end_row();
                            ui.add(egui::Label::new("p"));
                            ui.label(format!("{}", cpu.p.bits()));
                            ui.label(format!("0x{:02X}", cpu.p.bits()));
                            ui.end_row();
                        });
                    let flags = [
                        (Flags::N, "N"),
                        (Flags::V, "V"),
                        (Flags::B, "B"),
                        (Flags::D, "D"),
                        (Flags::I, "I"),
                        (Flags::Z, "Z"),
                        (Flags::C, "C"),
                    ];
                    egui::Grid::new("flags_grid")
                        .num_columns(flags.len())
                        .striped(true)
                        .show(ui, |ui| {
                            for flag in &flags {
                                ui.label(flag.1);
                            }
                            ui.end_row();
                            for flag in flags {
                                if cpu.p.contains(flag.0) {
                                    ui.label("✔");
                                } else {
                                    ui.label("-");
                                }
                            }
                            ui.end_row();
                        });
                }

                ui.separator();

                ui.heading("Stack");
                if let Ok(bus) = self.debug_state.bus.read() {
                    egui::ScrollArea::vertical()
                        .max_height(200.0)
                        .show(ui, |ui| {
                            egui::Grid::new("sp_grid")
                                .num_columns(3)
                                .striped(true)
                                .show(ui, |ui| {
                                    for i in (0x100..0x1FF).rev().into_iter() {
                                        ui.add(egui::Label::new(format!("0x{:04X}", i)));
                                        ui.label(format!("{}", bus.read_byte(i)));
                                        ui.label(format!("0x{:02X}", bus.read_byte(i)));
                                        ui.end_row();
                                    }
                                });
                        });
                }

                ui.heading("Memory");
                egui::TextEdit::singleline(&mut self.mem_search)
                    .hint_text("Address")
                    .char_limit(4)
                    .show(ui);
                if let Ok(bus) = self.debug_state.bus.read() {
                    egui::Grid::new("mem_grid")
                        .num_columns(3)
                        .striped(true)
                        .show(ui, |ui| {
                            const SHOW_MORE: usize = 5;

                            if let Ok(n) = usize::from_str_radix(&self.mem_search, 16) {
                                for i in n.saturating_sub(SHOW_MORE)..=(n + SHOW_MORE).min(0xffff) {
                                    ui.add(egui::Label::new(format!("0x{:04X}", i)));
                                    ui.label(format!("{}", bus.read_byte(i)));
                                    ui.label(format!("0x{:02X}", bus.read_byte(i)));
                                    ui.end_row();
                                }
                            }
                        });
                }
            });
        egui::SidePanel::right("right_panel")
            .resizable(true)
            .default_width(200.0)
            .show(ctx, |ui| {
                ui.heading("Loaded ROM");
                if let Ok(cart_header_opt) = self.debug_state.cart_header.read() {
                    match &*cart_header_opt {
                        Some(cart_header) => {
                            ui.label(
                                String::from_utf8(cart_header.magic.to_vec())
                                    .unwrap_or("".to_string())
                                    .to_string(),
                            );
                            ui.label(format!("has trainer: {}", cart_header.flags6.has_trainer()));
                            ui.label(format!("mapper: {:?}", cart_header.get_mapper()));
                            ui.label(format!(
                                "prg rom size {}",
                                ByteSize::kib(16) * cart_header.prg_rom_size
                            ));
                        }
                        None => {
                            ui.label("Not loaded");
                        }
                    }
                }
            });
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    if let Ok(cpu_log) = self.debug_state.cpu_log.read() {
                        ui.label(
                            egui::RichText::new(&*cpu_log).text_style(egui::TextStyle::Monospace),
                        );
                    }
                })
        });
    }
}
