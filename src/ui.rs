use bytesize::ByteSize;
use log::error;
use std::sync::{Arc, mpsc};

use crate::{cpu::Flags, debug::DebugState, emu::Command};

pub struct Ui {
    command_tx: mpsc::Sender<Command>,

    debug_state: Arc<DebugState>,

    mem_search: String,
}

impl Ui {
    pub fn new(command_tx: mpsc::Sender<Command>, debug_state: Arc<DebugState>) -> Self {
        Self {
            command_tx: command_tx,
            debug_state: debug_state,
            mem_search: "".to_string(),
        }
    }

    fn send_command(&self, command: Command) {
        if let Err(e) = self.command_tx.send(command) {
            error!("{e}");
        }
    }

    pub fn draw(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
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
        egui::CentralPanel::default().show(ctx, |_ui| {});
        egui::SidePanel::left("left_panel")
            .resizable(false)
            .show(ctx, |ui| {
                if let Ok(cpu) = self.debug_state.cpu.read() {
                    ui.heading("CPU");
                    egui::Grid::new("cpu_grid")
                        .num_columns(2)
                        .striped(true)
                        .show(ui, |ui| {
                            ui.add(egui::Label::new("sp"));
                            ui.label(format!("{}", cpu.sp));
                            ui.end_row();
                            ui.add(egui::Label::new("pc"));
                            ui.label(format!("{}", cpu.pc));
                            ui.end_row();
                            ui.add(egui::Label::new("a"));
                            ui.label(format!("{}", cpu.a));
                            ui.end_row();
                            ui.add(egui::Label::new("x"));
                            ui.label(format!("{}", cpu.x));
                            ui.end_row();
                            ui.add(egui::Label::new("y"));
                            ui.label(format!("{}", cpu.y));
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
                                if cpu.flags.contains(flag.0) {
                                    ui.label("✔");
                                } else {
                                    ui.label("-");
                                }
                            }
                            ui.end_row();
                        });
                }

                ui.separator();

                ui.heading("Memory");
                egui::TextEdit::singleline(&mut self.mem_search)
                    .hint_text("Address")
                    .char_limit(4)
                    .show(ui);
                if let Ok(bus) = self.debug_state.bus.read() {
                    egui::Grid::new("mem_grid")
                        .num_columns(2)
                        .striped(true)
                        .show(ui, |ui| {
                            if let Ok(n) = self.mem_search.parse::<usize>() {
                                ui.add(egui::Label::new(format!("0x{}", self.mem_search)));
                                ui.label(format!("{}", bus.read_byte(n)));
                                ui.end_row();
                            }
                        });
                }
            });
        egui::SidePanel::right("right_panel")
            .resizable(false)
            .show(ctx, |ui| {
                ui.heading("Loaded ROM");
                if let Ok(cart_header_opt) = self.debug_state.cart_header.read() {
                    match &*cart_header_opt {
                        Some(cart_header) => {
                            ui.label(format!(
                                "{}",
                                String::from_utf8(cart_header.magic.to_vec())
                                    .unwrap_or("".to_string())
                            ));
                            ui.label(format!("has trainer: {}", cart_header.flags6.has_trainer));
                            ui.label(format!(
                                "prg_size {}",
                                ByteSize::kib(16) * cart_header.prg_rom_size
                            ));
                        }
                        None => {
                            ui.label("Not loaded");
                        }
                    }
                }
            });
    }
}
