use std::sync::Arc;

use crate::{app::App, debug::DebugState, emu::Command};

pub struct Ui {
    debug_state: Arc<DebugState>,
}

impl Ui {
    pub fn new(debug_state: Arc<DebugState>) -> Self {
        Self {
            debug_state: debug_state,
        }
    }

    pub fn draw(&self, app: &App, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("menubar").show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Quit").clicked() {
                        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
                ui.menu_button("Emulator", |ui| {
                    if ui.button("Start").clicked() {
                        app.send_command(Command::Start);
                    }
                    if ui.button("Stop").clicked() {
                        app.send_command(Command::Stop);
                    }
                });
            });
        });
        egui::CentralPanel::default().show(ctx, |ui| {});
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
                            ui.add(egui::Label::new("p"));
                            ui.label(format!("{}", cpu.flags));
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
                }
                // ui.text_edit_singleline(&mut self.mem_search);
                // if let Ok(bus) = self.debug_state.bus.read() {
                //     egui::Grid::new("mem_grid")
                //         .num_columns(2)
                //         .striped(true)
                //         .show(ui, |ui| {
                //             ui.add(egui::Label::new(format!("{}", self.mem_search)));
                //             ui.label(format!("{}", bus.read_byte(self.mem_search)));
                //             ui.end_row();
                //         });
                // }
            });
    }
}
