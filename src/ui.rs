use bytesize::ByteSize;
use egui_extras::{Column, TableBuilder};
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

    fn draw_menubar(&mut self, ui: &mut egui::Ui) {
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("✖ Quit").clicked() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
            ui.menu_button("Emulator", |ui| {
                ui.add_enabled_ui(self.paused, |ui| {
                    if ui.button("⤵ Step").clicked() {
                        self.emu_step();
                    }
                });
                ui.add_enabled_ui(self.paused, |ui| {
                    if ui.button("⏸ Resume").clicked() {
                        self.emu_resume();
                    }
                });
                ui.add_enabled_ui(!self.paused, |ui| {
                    if ui.button("⏵ Pause").clicked() {
                        self.emu_pause();
                    }
                });
                ui.add_enabled_ui(self.running, |ui| {
                    if ui.button("⏹ Stop").clicked() {
                        self.emu_stop();
                    }
                });
            });
        });
    }

    fn draw_memory_viewer(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            let n = (match self.mem_search.starts_with("0x") {
                true => usize::from_str_radix(&self.mem_search[2..], 16).unwrap_or(0),
                false => self.mem_search.parse::<usize>().unwrap_or(0),
            })
            .min(0xffff);

            ui.label(
                egui::RichText::new(format!("{} 0x{:04X}", n, n))
                    .strong()
                    .text_style(egui::TextStyle::Monospace),
            );

            if let Ok(bus) = self.debug_state.bus.read() {
                TableBuilder::new(ui)
                    .striped(true)
                    .column(Column::auto())
                    .column(Column::auto())
                    .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                    .body(|mut body| {
                        let lo = n & 0xfff0;
                        let bytes_per_row = 0x10;
                        let total_rows = 7;
                        let max_rows_above = lo / bytes_per_row;
                        let rows_above = max_rows_above.min(total_rows - 1);
                        let rows_below = total_rows - 1 - rows_above;
                        let start = lo - (rows_above * bytes_per_row);
                        let end = (lo + rows_below * bytes_per_row).min(0xffff);

                        for i in (start..=end).step_by(bytes_per_row) {
                            let bytes_str: Vec<String> = bus
                                .read(i as u16, 0x10)
                                .to_vec()
                                .iter()
                                .map(|b| format!("{:02X}", b))
                                .collect();

                            body.row(20.0, |mut row| {
                                row.col(|ui| {
                                    ui.label(
                                        egui::RichText::new(format!("0x{:04X}", i))
                                            .strong()
                                            .text_style(egui::TextStyle::Monospace),
                                    );
                                });
                                row.col(|ui| {
                                    ui.label(
                                        egui::RichText::new(bytes_str.join(" "))
                                            .text_style(egui::TextStyle::Monospace),
                                    );
                                });
                            });
                        }
                    });
                ui.shrink_width_to_current();
                egui::TextEdit::singleline(&mut self.mem_search)
                    .hint_text("...")
                    .char_limit(8)
                    .desired_width(f32::INFINITY)
                    .show(ui);
            }
        });
    }

    fn draw_cpu_inspector(&mut self, ui: &mut egui::Ui) {
        if let Ok(cpu) = self.debug_state.cpu.read() {
            ui.label(egui::RichText::new("CPU").strong());
            TableBuilder::new(ui)
                .id_salt("cpu")
                .striped(true)
                .column(Column::auto())
                .column(Column::auto())
                .column(Column::remainder())
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                .body(|mut body| {
                    body.row(16.0, |mut row| {
                        row.col(|ui| {
                            ui.label(egui::RichText::new("sp").strong());
                        });
                        row.col(|ui| {
                            ui.label(format!("{}", cpu.sp + 0x100));
                        });
                        row.col(|ui| {
                            ui.label(format!("0x{:04X}", cpu.sp + 0x100));
                        });
                    });
                    body.row(16.0, |mut row| {
                        row.col(|ui| {
                            ui.label(egui::RichText::new("pc").strong());
                        });
                        row.col(|ui| {
                            ui.label(format!("{}", cpu.pc));
                        });
                        row.col(|ui| {
                            ui.label(format!("0x{:04X}", cpu.pc));
                        });
                    });
                    body.row(16.0, |mut row| {
                        row.col(|ui| {
                            ui.label(egui::RichText::new("a").strong());
                        });
                        row.col(|ui| {
                            ui.label(format!("{}", cpu.a));
                        });
                        row.col(|ui| {
                            ui.label(format!("0x{:02X}", cpu.a));
                        });
                    });
                    body.row(16.0, |mut row| {
                        row.col(|ui| {
                            ui.label(egui::RichText::new("x").strong());
                        });
                        row.col(|ui| {
                            ui.label(format!("{}", cpu.x));
                        });
                        row.col(|ui| {
                            ui.label(format!("0x{:02X}", cpu.x));
                        });
                    });
                    body.row(16.0, |mut row| {
                        row.col(|ui| {
                            ui.label(egui::RichText::new("y").strong());
                        });
                        row.col(|ui| {
                            ui.label(format!("{}", cpu.y));
                        });
                        row.col(|ui| {
                            ui.label(format!("0x{:02X}", cpu.y));
                        });
                    });
                    body.row(16.0, |mut row| {
                        row.col(|ui| {
                            ui.label(egui::RichText::new("p").strong());
                        });
                        row.col(|ui| {
                            ui.label(format!("{}", cpu.p.bits()));
                        });
                        row.col(|ui| {
                            ui.label(format!("0x{:02X}", cpu.p.bits()));
                        });
                    });
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
            TableBuilder::new(ui)
                .id_salt("flags")
                .column(Column::remainder())
                .column(Column::remainder())
                .column(Column::remainder())
                .column(Column::remainder())
                .column(Column::remainder())
                .column(Column::remainder())
                .column(Column::remainder())
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                .body(|mut body| {
                    body.row(14.0, |mut row| {
                        for flag in &flags {
                            row.col(|ui| {
                                ui.label(flag.1);
                            });
                        }
                    });
                    body.row(14.0, |mut row| {
                        for flag in flags {
                            if cpu.p.contains(flag.0) {
                                row.col(|ui| {
                                    ui.label("✔");
                                });
                            } else {
                                row.col(|ui| {
                                    ui.label("-");
                                });
                            }
                        }
                    })
                });
        }

        ui.label(egui::RichText::new("Stack").strong());
        if let Ok(bus) = self.debug_state.bus.read() {
            egui::ScrollArea::vertical() // FIXME: optimize this
                .max_height(200.0)
                .show(ui, |ui| {
                    TableBuilder::new(ui)
                        .id_salt("cpu")
                        .striped(true)
                        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                        .column(Column::auto())
                        .column(Column::auto())
                        .column(Column::remainder())
                        .body(|mut body| {
                            for i in (0x100..0x1FF).rev() {
                                body.row(16.0, |mut row| {
                                    row.col(|ui| {
                                        ui.label(format!("0x{:04X}", i));
                                    });
                                    row.col(|ui| {
                                        ui.label(format!("{}", bus.read_byte(i)));
                                    });
                                    row.col(|ui| {
                                        ui.label(format!("0x{:02X}", bus.read_byte(i)));
                                    });
                                });
                            }
                        });
                });
        }
    }

    fn draw_rom_details(&mut self, ui: &mut egui::Ui) {
        ui.label(egui::RichText::new("ROM details").strong());
        if let Ok(cart_header_opt) = self.debug_state.cart_header.read() {
            match &*cart_header_opt {
                Some(cart_header) => {
                    TableBuilder::new(ui)
                        .striped(true)
                        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                        .column(Column::auto())
                        .column(Column::remainder())
                        .body(|mut body| {
                            body.row(16.0, |mut row| {
                                row.col(|ui| {
                                    ui.label(egui::RichText::new("Magic").strong());
                                });
                                row.col(|ui| {
                                    ui.label(
                                        String::from_utf8(cart_header.magic.to_vec())
                                            .unwrap_or("".to_string())
                                            .to_string(),
                                    );
                                });
                            });
                            body.row(16.0, |mut row| {
                                row.col(|ui| {
                                    ui.label(egui::RichText::new("Trainer?").strong());
                                });
                                row.col(|ui| {
                                    ui.label("✔");
                                });
                            });
                            body.row(16.0, |mut row| {
                                row.col(|ui| {
                                    ui.label(egui::RichText::new("PRG ROM Size").strong());
                                });
                                row.col(|ui| {
                                    ui.label(format!(
                                        "{}",
                                        ByteSize::kib(16) * cart_header.prg_rom_size
                                    ));
                                });
                            });
                            body.row(16.0, |mut row| {
                                row.col(|ui| {
                                    ui.label(egui::RichText::new("Mapper").strong());
                                });
                                row.col(|ui| {
                                    ui.label(format!("{}", cart_header.get_mapper()));
                                });
                            });
                        });
                }
                None => {
                    ui.label("Not loaded");
                }
            }
        }
    }

    fn draw_log_reader(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical()
            .stick_to_bottom(true)
            .auto_shrink(false)
            .show(ui, |ui| {
                if let Ok(cpu_log) = self.debug_state.cpu_log.read() {
                    let start = cpu_log.len() - 3000;
                    let slice = &cpu_log[start..];
                    ui.label(egui::RichText::new(slice).text_style(egui::TextStyle::Monospace));
                }
            });
    }

    pub fn draw(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("menubar")
            .resizable(false)
            .show(ctx, |ui| {
                self.draw_menubar(ui);
            });
        egui::TopBottomPanel::bottom("bottom_panel")
            .resizable(false)
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    self.draw_memory_viewer(ui);
                });
            });
        egui::SidePanel::left("left_panel")
            .resizable(false)
            .default_width(180.0)
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    self.draw_cpu_inspector(ui);
                });
            });
        egui::SidePanel::right("right_panel")
            .resizable(true)
            .default_width(180.0)
            .width_range(80.0..=200.0)
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    self.draw_rom_details(ui);
                });
            });
        egui::CentralPanel::default().show(ctx, |ui| {
            self.draw_log_reader(ui);
        });
        ctx.request_repaint();
    }
}
