use bytesize::ByteSize;
use egui::{Color32, Key};
use egui_extras::{Column, TableBuilder};
use log::{error, info};
#[cfg(not(target_arch = "wasm32"))]
use rfd::FileDialog;
use std::{
    path::Path,
    sync::{Arc, mpsc},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use crate::{
    args::Args,
    cpu::Flags,
    debug::{BYTES_PER_ROW, DebugState, ROWS_TO_SHOW},
    emu::{Command, Event, emu_thread},
    mapper::MapperIcon,
};

macro_rules! make_rows {
    ($body:expr, $( $label:expr => $value:expr ),+ $(,)?) => {
        $(
            $body.row(16.0, |mut row| {
                row.col(|ui| {
                    ui.label(egui::RichText::new($label).strong());
                });
                row.col(|ui| {
                    ui.label($value);
                });
            });
        )+
    };

    ($body:expr, $( $label:expr => $value1:expr, $value2:expr ),+ $(,)?) => {
        $(
            $body.row(16.0, |mut row| {
                row.col(|ui| {
                    ui.label(egui::RichText::new($label).strong());
                });
                row.col(|ui| {
                    ui.label($value1);
                });
                row.col(|ui| {
                    ui.label($value2);
                });
            });
        )+
    };
}

pub struct FrameStats {
    last_update: Instant,
    frame_count: u64,
    fps: f32,
    target_interval: Duration,
    fps_last_update: Instant,
    render_count_since_fps_update: u64,
}

impl FrameStats {
    pub fn new(target_fps: f32) -> Self {
        let target_interval_secs = 1.0 / target_fps;
        let target_interval = Duration::from_secs_f32(target_interval_secs);
        let now = Instant::now();

        Self {
            last_update: now,
            frame_count: 0,
            fps: 0.0,
            target_interval,
            fps_last_update: now,
            render_count_since_fps_update: 0,
        }
    }

    pub fn should_render(&mut self) -> bool {
        let next_update_time = self.last_update.checked_add(self.target_interval);

        if let Some(next_update) = next_update_time {
            let now = Instant::now();
            if now >= next_update {
                self.frame_count += 1;
                self.render_count_since_fps_update += 1;
                self.last_update = now;
                return true;
            }
        }
        false
    }

    pub fn update_fps(&mut self) {
        let elapsed_duration = self.fps_last_update.elapsed();
        let one_second = Duration::from_secs(1);

        if elapsed_duration >= one_second {
            let elapsed_secs = elapsed_duration.as_secs_f32();

            self.fps = self.render_count_since_fps_update as f32 / elapsed_secs;

            self.render_count_since_fps_update = 0;
            self.fps_last_update = Instant::now();
        }
    }
}

#[derive(Default)]
pub struct Screen {
    pub width: usize,
    pub height: usize,
    pixels: Vec<Color32>,
    pub texture_handle: Option<egui::TextureHandle>,
}

impl Screen {
    pub fn new() -> Self {
        let width: usize = 256;
        let height: usize = 240;
        let pixels = vec![Color32::BLACK; width * height];
        Self {
            width,
            height,
            pixels,
            texture_handle: None,
        }
    }

    pub fn update_texture(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        let image = egui::ColorImage::new([self.width, self.height], self.pixels.clone());

        if let Some(texture) = &mut self.texture_handle {
            texture.set(image, egui::TextureOptions::NEAREST);
        } else {
            self.texture_handle =
                Some(ctx.load_texture("screen", image, egui::TextureOptions::NEAREST));
        }

        let texture = self.texture_handle.as_ref().unwrap();
        let available = ui.available_size();
        let aspect_ratio = self.width as f32 / self.height as f32;

        let fitted_size = if available.x / available.y > aspect_ratio {
            egui::Vec2::new(available.y * aspect_ratio, available.y)
        } else {
            egui::Vec2::new(available.x, available.x / aspect_ratio)
        };

        ui.image((texture.id(), fitted_size));
    }
}

#[derive(Default)]
pub struct Input {
    a: bool,
    b: bool,
    select: bool,
    start: bool,
    up: bool,
    down: bool,
    left: bool,
    right: bool,
}

impl Input {
    pub fn as_byte(&self) -> u8 {
        let mut byte: u8 = 0;

        if self.a {
            byte |= 1 << 0;
        }
        if self.b {
            byte |= 1 << 1;
        }
        if self.select {
            byte |= 1 << 2;
        }
        if self.start {
            byte |= 1 << 3;
        }
        if self.up {
            byte |= 1 << 4;
        }
        if self.down {
            byte |= 1 << 5;
        }
        if self.left {
            byte |= 1 << 6;
        }
        if self.right {
            byte |= 1 << 7;
        }

        byte
    }
}

pub struct Ui {
    screen: Screen,
    command_tx: Option<mpsc::Sender<Command>>,
    event_rx: Option<mpsc::Receiver<Event>>,
    args: Args,
    pub debug_state: Arc<DebugState>,

    emu_thread_handle: Option<JoinHandle<()>>,
    emu_error_msg: Option<String>,

    mem_search: String,
    prev_mem_search_addr: usize,

    show_about: bool,
    show_debug_panels: bool,

    running: bool,
    paused: bool,
    frame_ready: bool,
    frame_stats: FrameStats,

    pixels_buffer: Option<Vec<Color32>>,

    controller1_input: Input,
    controller2_input: Input,
}

impl Ui {
    pub fn new(debug_state: Arc<DebugState>, args: Args) -> Self {
        Self {
            screen: Screen::new(),
            command_tx: None,
            event_rx: None,
            args,
            debug_state,
            emu_thread_handle: None,
            emu_error_msg: None,
            mem_search: "".into(),
            prev_mem_search_addr: 0,
            show_about: false,
            show_debug_panels: true,
            running: false,
            paused: false,
            frame_ready: false,
            frame_stats: FrameStats::new(60.0),
            pixels_buffer: None,
            controller1_input: Default::default(),
            controller2_input: Default::default(),
        }
    }

    pub fn process_input(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if !ctx.wants_keyboard_input() {
            ctx.input_mut(|i| {
                self.controller1_input.a = i.key_down(Key::A);
                self.controller1_input.b = i.key_down(Key::B);
                self.controller1_input.start = i.key_down(Key::Z);
                self.controller1_input.select = i.key_down(Key::N);
                self.controller1_input.up = i.key_down(Key::ArrowUp);
                self.controller1_input.down = i.key_down(Key::ArrowDown);
                self.controller1_input.left = i.key_down(Key::ArrowLeft);
                self.controller1_input.right = i.key_down(Key::ArrowRight);

                // TODO
                // self.controller2_input.a = i.key_down(Key::A);
                // self.controller2_input.b = i.key_down(Key::B);
                // self.controller2_input.start = i.key_down(Key::Z);
                // self.controller2_input.select = i.key_down(Key::N);
                // self.controller2_input.up = i.key_down(Key::ArrowUp);
                // self.controller2_input.down = i.key_down(Key::ArrowDown);
                // self.controller2_input.left = i.key_down(Key::ArrowLeft);
                // self.controller2_input.right = i.key_down(Key::ArrowRight);
            });
        }
        self.send_command(Command::ControllerInputs(
            (self.controller2_input.as_byte() as u16) << 8
                | self.controller1_input.as_byte() as u16,
        ));
    }

    fn stop_emu_thread(&mut self) {
        if let Some(command_tx) = self.command_tx.take() {
            let _ = command_tx.send(Command::Stop);

            if let Some(handle) = self.emu_thread_handle.take() {
                let _ = handle.join();
            }
        }

        self.event_rx = None;
        self.running = false;
        self.paused = false;
    }

    pub fn spawn_emu_thread(&mut self, rom: &str) {
        if self.running {
            self.stop_emu_thread();
        }

        let args = self.args.clone();
        let rom = rom.to_owned();
        let pause = args.pause;
        let debug_state = self.debug_state.clone();

        let (command_tx, command_rx) = mpsc::channel();
        let (event_tx, event_rx) = mpsc::channel();

        self.command_tx = Some(command_tx);
        self.event_rx = Some(event_rx);
        let event_tx_clone = event_tx.clone();

        self.emu_error_msg = None;

        let handle = thread::Builder::new()
            .name("emu_thread".to_string())
            .spawn(move || {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    emu_thread(command_rx, event_tx, debug_state, &args, &rom);
                }));

                if let Err(e) = result {
                    error!("Emulator thread panicked: {:?}", e);
                    _ = event_tx_clone.send(Event::Crashed(format!("{:?}", e)));
                }
            })
            .expect("Failed to spawn emu thread");

        self.emu_thread_handle = Some(handle);
        self.running = true;
        self.paused = pause;
    }

    fn send_command(&self, command: Command) {
        if let Some(command_tx) = &self.command_tx
            && let Err(e) = command_tx.send(command)
        {
            error!("{e}");
        }
    }

    pub fn emu_step(&mut self) {
        if self.running && self.paused {
            self.send_command(Command::Step);
        }
    }

    pub fn emu_resume(&mut self) {
        if self.running && self.paused {
            self.send_command(Command::Resume);
        }
    }

    pub fn emu_pause(&mut self) {
        if self.running && !self.paused {
            self.send_command(Command::Pause);
        }
    }

    pub fn emu_stop(&mut self) {
        if self.running {
            self.stop_emu_thread();
        }
    }

    pub fn is_paused(&self) -> bool {
        self.paused
    }

    fn draw_menubar(&mut self, ui: &mut egui::Ui) {
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button("File", |ui| {
                #[cfg(not(target_arch = "wasm32"))]
                // TODO: implement wasm file dialog (https://github.com/PolyMeilex/rfd/blob/master/examples/web-trunk/src/main.rs)
                if ui.button("📥 Select rom...").clicked()
                    && let Some(rom) = FileDialog::new()
                        .add_filter("NES rom", &["nes"])
                        .pick_file()
                {
                    self.spawn_emu_thread(&rom.into_os_string().into_string().unwrap());
                }
                ui.separator();
                if ui.button("✖ Quit").clicked() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
            ui.menu_button("Emulator", |ui| {
                ui.add_enabled_ui(self.running && self.paused, |ui| {
                    if ui.button("⤵ Step").clicked() {
                        self.emu_step();
                    }
                });
                ui.add_enabled_ui(self.running && self.paused, |ui| {
                    if ui.button("▶ Resume").clicked() {
                        self.emu_resume();
                    }
                });
                ui.add_enabled_ui(self.running && !self.paused, |ui| {
                    if ui.button("⏸ Pause").clicked() {
                        self.emu_pause();
                    }
                });
                ui.add_enabled_ui(self.running, |ui| {
                    if ui.button("⏹ Stop").clicked() {
                        self.emu_stop();
                    }
                });
                ui.separator();
                if ui.button("📷 Take snapshot").clicked() {
                    self.take_snapshot();
                }
                ui.checkbox(&mut self.show_debug_panels, "Show debug panels");
            });
            ui.menu_button("Help", |ui| {
                if ui.button("ℹ About").clicked() {
                    self.show_about = true;
                }
            });
            if self.show_about {
                let modal = egui::Modal::new(egui::Id::new("about_modal")).show(ui.ctx(), |ui| {
                    ui.set_width(320.0);
                    ui.heading("Nessie");
                    ui.horizontal(|ui| {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("Close").clicked() {
                                ui.close();
                            }
                        });
                    });
                });
                if modal.should_close() {
                    self.show_about = false;
                }
            }
        });
    }

    fn show_input(&self, ui: &mut egui::Ui) {
        ui.label(egui::RichText::new("Controller").strong());
        TableBuilder::new(ui)
            .id_salt("controller")
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(Column::auto())
            .column(Column::remainder())
            .body(|mut body| {
                make_rows!(body,
                    "A" => format!("{}", self.controller1_input.a),
                    "B" => format!("{}", self.controller1_input.b),
                    "Select" => format!("{}", self.controller1_input.select),
                    "Start" => format!("{}", self.controller1_input.start),
                    "Up" => format!("{}", self.controller1_input.up),
                    "Down" => format!("{}", self.controller1_input.down),
                    "Left" => format!("{}", self.controller1_input.left),
                    "Right" => format!("{}", self.controller1_input.right),
                );
            });
    }

    fn draw_memory_viewer(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            let n = (match self.mem_search.starts_with("0x") {
                true => usize::from_str_radix(&self.mem_search[2..], 16).unwrap_or(0),
                false => self.mem_search.parse::<usize>().unwrap_or(0),
            })
            .min(0xffff);

            const ROWS_ABOVE_CURRENT: usize = ROWS_TO_SHOW / 2;
            const MAX_ROW: usize = 0xFFFF / BYTES_PER_ROW;

            let current_row = (n & 0xFFF0) / BYTES_PER_ROW;
            let mut start_row = current_row.saturating_sub(ROWS_ABOVE_CURRENT);
            if start_row + ROWS_TO_SHOW - 1 > MAX_ROW {
                start_row = (MAX_ROW + 1).saturating_sub(ROWS_TO_SHOW);
            }
            let start_addr = start_row * BYTES_PER_ROW;
            if start_addr != self.prev_mem_search_addr {
                self.send_command(Command::MemoryAddress(start_addr));
                self.prev_mem_search_addr = start_addr;
            }

            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(format!("0x{:04X} {}", n, n))
                        .strong()
                        .text_style(egui::TextStyle::Monospace),
                );
                if ui.button("Dump").clicked() {
                    self.send_command(Command::DumpMemory);
                }
            });

            if let Ok(mem_chunk) = self.debug_state.mem_chunk.read() {
                TableBuilder::new(ui)
                    .striped(true)
                    .column(Column::auto())
                    .column(Column::auto())
                    .column(Column::auto())
                    .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                    .body(|mut body| {
                        for (i, lines) in mem_chunk
                            .chunks(BYTES_PER_ROW)
                            .take(ROWS_TO_SHOW)
                            .enumerate()
                        {
                            let bytes_str: Vec<String> =
                                lines.iter().map(|b| format!("{:02X}", b)).collect();
                            let bytes_ascii: Vec<char> = lines
                                .iter()
                                .map(|b| {
                                    if !b.is_ascii_graphic() {
                                        '.'
                                    } else {
                                        *b as char
                                    }
                                })
                                .collect();

                            let row_addr = start_addr + (i * BYTES_PER_ROW);

                            body.row(20.0, |mut row| {
                                row.col(|ui| {
                                    ui.label(
                                        egui::RichText::new(format!("0x{:04X}", row_addr))
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
                                row.col(|ui| {
                                    ui.label(
                                        egui::RichText::new(
                                            String::from_iter(bytes_ascii).to_string(),
                                        )
                                        .text_style(egui::TextStyle::Monospace),
                                    );
                                });
                            });
                        }
                    });
                ui.shrink_width_to_current();
                egui::TextEdit::singleline(&mut self.mem_search)
                    .hint_text("27, 0xD0D0, ...")
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
                    make_rows!(body,
                        "sp" => format!("{}", cpu.sp as usize + 0x100), format!("0x{:04X}", cpu.sp as usize + 0x100),
                        "pc" => format!("{}", cpu.pc), format!("0x{:04X}", cpu.pc),
                        "a" => format!("{}", cpu.a), format!("0x{:02X}", cpu.a),
                        "x" => format!("{}", cpu.x), format!("0x{:02X}", cpu.x),
                        "y" => format!("{}", cpu.y), format!("0x{:02X}", cpu.y),
                        "p" => format!("{}", cpu.p.bits()), format!("0x{:02X}", cpu.p.bits()),
                    );
                });

            let mut flags = [
                (Flags::N, "N", false),
                (Flags::V, "V", false),
                (Flags::B, "B", false),
                (Flags::D, "D", false),
                (Flags::I, "I", false),
                (Flags::Z, "Z", false),
                (Flags::C, "C", false),
            ];

            egui::CollapsingHeader::new("Flags").show(ui, |ui| {
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
                        body.row(16.0, |mut row| {
                            for flag in &mut flags {
                                flag.2 = cpu.p.contains(flag.0);
                                row.col(|ui| {
                                    ui.add_enabled(false, egui::Checkbox::new(&mut flag.2, flag.1));
                                });
                            }
                        })
                    });
            });
        }

        egui::CollapsingHeader::new("Stack").show(ui, |ui| {
            if let Ok(stack) = self.debug_state.stack.read() {
                egui::ScrollArea::vertical()
                    .max_height(200.0)
                    .show(ui, |ui| {
                        TableBuilder::new(ui)
                            .id_salt("stack")
                            .striped(true)
                            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                            .column(Column::auto())
                            .column(Column::auto())
                            .column(Column::remainder())
                            .body(|mut body| {
                                for (i, n) in stack.iter().enumerate().rev() {
                                    make_rows!(body,
                                        format!("0x{:04X}", i + 0x100) =>
                                            format!("{}", n),
                                            format!("0x{:02X}",n));
                                }
                            });
                    });
            }
        });
    }

    fn draw_ppu_inspector(&mut self, ui: &mut egui::Ui) {
        if let Ok(ppu) = self.debug_state.ppu.read() {
            ui.label(egui::RichText::new("PPU").strong());
            egui::ScrollArea::vertical()
                .auto_shrink(false)
                .show(ui, |ui| {
                    TableBuilder::new(ui)
                        .id_salt("ppu")
                        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                        .column(Column::auto())
                        .column(Column::remainder())
                        .body(|mut body| {
                            make_rows!(body,
                                "Dot" => format!("{}", ppu.dot),
                                "Scanline" => format!("{}", ppu.scanline),
                                "Frame" => format!("{}", ppu.frame),
                            );
                        });
                    egui::CollapsingHeader::new("PPU Ctrl ($2000)").show(ui, |ui| {
                        TableBuilder::new(ui)
                            .id_salt("ppuctrl")
                            .striped(true)
                            .column(Column::auto())
                            .column(Column::remainder())
                            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                            .body(|mut body| {
                                let base_nt_addr = match ppu.ctrl.base_nametable_addr() {
                                    0 => 0x2000,
                                    1 => 0x2400,
                                    2 => 0x2800,
                                    _ => 0x2C00,
                                };
                                let vram_inc = if ppu.ctrl.vram_addr_inc() != 0 { 32 } else { 1 };
                                let sprite_pt_addr = if ppu.ctrl.sprite_pattern_table() != 0 {
                                    0x1000
                                } else {
                                    0x0000
                                };
                                let bg_pt_addr = if ppu.ctrl.bg_pattern_table() != 0 {
                                    0x1000
                                } else {
                                    0x0000
                                };
                                let sprite_size = if ppu.ctrl.sprite_size() != 0 {
                                    "8x16"
                                } else {
                                    "8x8"
                                };
                                let master_slave = if ppu.ctrl.master_slave() != 0 {
                                    "Slave"
                                } else {
                                    "Master"
                                };

                                make_rows!(body,
                                    "Base nametable" => format!("0x{:04X}", base_nt_addr),
                                    "VRAM addr increment" => format!("{}", vram_inc),
                                    "Sprite pattern table" => format!("0x{:04X}", sprite_pt_addr),
                                    "BG pattern table" => format!("0x{:04X}", bg_pt_addr),
                                    "Sprite size" => format!("{}", sprite_size),
                                    "Master/slave" => format!("{}", master_slave),
                                    "NMI enable" => format!("{}", ppu.ctrl.nmi_enable())
                                );
                            });
                    });
                    egui::CollapsingHeader::new("PPU Mask ($2001)").show(ui, |ui| {
                        TableBuilder::new(ui)
                        .id_salt("ppumask")
                        .striped(true)
                        .column(Column::auto())
                        .column(Column::remainder())
                        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                        .body(|mut body| {
                            make_rows!(body,
                                "Greyscale" => format!("{}", ppu.mask.greyscale()),
                                "Show BG left" => format!("{}", ppu.mask.show_bg_left()),
                                "Show sprites left" => format!("{}", ppu.mask.show_sprites_left()),
                                "Show BG" => format!("{}", ppu.mask.show_bg()),
                                "Show sprites" => format!("{}", ppu.mask.show_sprites()),
                                "Emphasize red" => format!("{}", ppu.mask.emphasize_red()),
                                "Emphasize green" => format!("{}", ppu.mask.emphasize_green()),
                                "Emphasize blue" => format!("{}", ppu.mask.emphasize_blue()),
                            );
                        });
                    });
                    egui::CollapsingHeader::new("PPU Status ($2002)").show(ui, |ui| {
                        TableBuilder::new(ui)
                        .id_salt("ppustatus")
                        .striped(true)
                        .column(Column::auto())
                        .column(Column::remainder())
                        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                        .body(|mut body| {
                            make_rows!(body,
                                "Sprite overflow" => format!("{}", ppu.status.sprite_overflow()),
                                "Sprite 0 hit" => format!("{}", ppu.status.sprite_0_hit()),
                                "VBlank" => format!("{}", ppu.status.vblank()),
                            );
                        });
                    });
                    egui::CollapsingHeader::new("OAM Address ($2003)").show(ui, |ui| {
                        TableBuilder::new(ui)
                            .id_salt("oamaddr")
                            .striped(true)
                            .column(Column::auto())
                            .column(Column::remainder())
                            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                            .body(|mut body| {
                                make_rows!(body,
                                    "OAM Address" => format!("0x{:02X}", ppu.oam_addr),
                                );
                            });
                    });
                    egui::CollapsingHeader::new("OAM Data ($2004)").show(ui, |ui| {
                        egui::ScrollArea::vertical()
                            .max_height(200.0)
                            .show(ui, |ui| {
                                TableBuilder::new(ui)
                                    .id_salt("oamdata")
                                    .striped(true)
                                    .column(Column::auto())
                                    .column(Column::remainder())
                                    .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                                    .body(|mut body| {
                                        for (i, chunk) in ppu.oam.chunks(4).enumerate() {
                                            if i >= 64 {
                                                break;
                                            }
                                            body.row(16.0, |mut row| {
                                                row.col(|ui| {
                                                    ui.label(
                                                        egui::RichText::new(format!(
                                                            "Sprite {}",
                                                            i
                                                        ))
                                                        .strong(),
                                                    );
                                                });
                                                row.col(|ui| {
                                                    ui.label(format!(
                                                        "Y:{} Tile:{:02X} Attr:{:02X} X:{}",
                                                        chunk[0], chunk[1], chunk[2], chunk[3]
                                                    ));
                                                });
                                            });
                                        }
                                    });
                            });
                    });
                    egui::CollapsingHeader::new("PPU Registers ($2005-$2007)").show(ui, |ui| {
                        TableBuilder::new(ui)
                            .id_salt("ppuregs")
                            .striped(true)
                            .column(Column::auto())
                            .column(Column::remainder())
                            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                            .body(|mut body| {
                                make_rows!(body,
                                    "v (current VRAM addr)" => format!("0x{:04X}", ppu.v),
                                    "t (temp VRAM addr)" => format!("0x{:04X}", ppu.t),
                                    "x (fine X scroll)" => format!("{}", ppu.x),
                                    "w (write toggle)" => format!("{}", ppu.w),
                                );
                            });
                    });

                    egui::CollapsingHeader::new("Palette RAM").show(ui, |ui| {
                        egui::ScrollArea::vertical()
                            .max_height(200.0)
                            .show(ui, |ui| {
                                TableBuilder::new(ui)
                                    .id_salt("palette_ram")
                                    .striped(false)
                                    .column(Column::auto())
                                    .column(Column::auto())
                                    .column(Column::auto())
                                    .column(Column::auto())
                                    .column(Column::auto())
                                    .column(Column::auto().at_least(10.0))
                                    .column(Column::auto())
                                    .column(Column::auto())
                                    .column(Column::auto())
                                    .column(Column::auto())
                                    .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                                    .body(|mut body| {
                                        for (row_idx, row_colors) in
                                            ppu.palette.chunks(8).enumerate()
                                        {
                                            let base_addr = row_idx * 8;
                                            body.row(16.0, |mut row| {
                                                row.col(|ui| {
                                                    ui.label(
                                                        egui::RichText::new(format!(
                                                            "${:04X}:",
                                                            0x3F00 + base_addr
                                                        ))
                                                        .strong()
                                                        .text_style(egui::TextStyle::Monospace),
                                                    );
                                                });
                                                for (i, &color_idx) in
                                                    row_colors.iter().take(4).enumerate()
                                                {
                                                    let palette_addr = base_addr + i;
                                                    let displayed_idx = match palette_addr {
                                                        0x10 | 0x14 | 0x18 | 0x1C => {
                                                            ppu.palette[palette_addr - 0x10]
                                                        }
                                                        _ => color_idx,
                                                    };
                                                    let color = ppu.get_color_from_palette(
                                                        displayed_idx & 0x3F,
                                                    );

                                                    let mut text = egui::RichText::new(format!(
                                                        "{:02X}",
                                                        color_idx
                                                    ))
                                                    .text_style(egui::TextStyle::Monospace);
                                                    if [0x10, 0x14, 0x18, 0x1C]
                                                        .contains(&palette_addr)
                                                    {
                                                        text = text.strikethrough();
                                                    }

                                                    row.col(|ui| {
                                                        ui.colored_label(color, text);
                                                    });
                                                }

                                                row.col(|ui| {
                                                    ui.label(
                                                        egui::RichText::new(format!(
                                                            "${:04X}:",
                                                            0x3F00 + base_addr + 4
                                                        ))
                                                        .strong()
                                                        .text_style(egui::TextStyle::Monospace),
                                                    );
                                                });
                                                for (i, &color_idx) in
                                                    row_colors.iter().skip(4).enumerate()
                                                {
                                                    let palette_addr = base_addr + 4 + i;
                                                    let displayed_idx = match palette_addr {
                                                        0x10 | 0x14 | 0x18 | 0x1C => {
                                                            ppu.palette[palette_addr - 0x10]
                                                        }
                                                        _ => color_idx,
                                                    };
                                                    let color = ppu.get_color_from_palette(
                                                        displayed_idx & 0x3F,
                                                    );

                                                    let mut text = egui::RichText::new(format!(
                                                        "{:02X}",
                                                        color_idx
                                                    ))
                                                    .text_style(egui::TextStyle::Monospace);
                                                    if [0x10, 0x14, 0x18, 0x1C]
                                                        .contains(&palette_addr)
                                                    {
                                                        text = text.strikethrough();
                                                    }

                                                    row.col(|ui| {
                                                        ui.colored_label(color, text);
                                                    });
                                                }
                                            });
                                        }
                                    });
                            });
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
                            make_rows!(body,
                                "Magic" =>
                                    String::from_utf8(cart_header.magic.to_vec())
                                        .unwrap_or("".to_string())
                                        .to_string(),
                                "Trainer?" =>
                                    format!("{}", cart_header.flags6.has_trainer()),
                                "PRG ROM Size" =>
                                    format!("{}", ByteSize::kib(16) * cart_header.prg_rom_size),
                                "CHR ROM Size" =>
                                    format!("{}", ByteSize::kib(8) * cart_header.chr_rom_size),
                            );
                            body.row(16.0, |mut row| {
                                row.col(|ui| {
                                    ui.label(egui::RichText::new("Mapper").strong());
                                });
                                row.col(|ui| {
                                    let mapper_num = cart_header.mapper_number();

                                    let icon = egui::Image::from_bytes(
                                        format!("mapper_icon_{}", mapper_num),
                                        MapperIcon::from_mapper_number(mapper_num).bytes(),
                                    );

                                    ui.add(icon);
                                    ui.label(format!("{}", cart_header.mapper_number()));
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

    fn draw_fps(&self, ui: &mut egui::Ui) {
        ui.label(format!("FPS: {:.1}", self.frame_stats.fps));
    }

    fn draw_log_reader(&self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            ui.label(egui::RichText::new("Log").strong());
            egui::ScrollArea::vertical()
                .stick_to_bottom(true)
                .auto_shrink(false)
                .show(ui, |ui| {
                    if let Ok(cpu) = self.debug_state.cpu.read()
                        && let Some(log) = cpu.log.as_ref()
                    {
                        ui.label(
                            egui::RichText::new(log.iter().collect::<String>())
                                .text_style(egui::TextStyle::Monospace),
                        );
                    }
                });
        });
    }

    fn draw_screen(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        if self.frame_ready && self.frame_stats.should_render() {
            self.frame_ready = false;
            if let Some(frame) = self.pixels_buffer.take() {
                self.screen.pixels = frame;
            }
        }
        self.frame_stats.update_fps();
        self.screen.update_texture(ctx, ui);
    }

    fn draw_start_screen(&self, _ctx: &egui::Context, ui: &mut egui::Ui) {
        match &self.emu_error_msg {
            Some(msg) => ui.colored_label(Color32::RED, msg),
            None => ui.label("Load a rom"),
        };
    }

    pub fn draw(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("menubar")
            .resizable(false)
            .show(ctx, |ui| {
                self.draw_menubar(ui);
            });
        if self.running {
            if self.show_debug_panels {
                egui::SidePanel::left("left_panel")
                    .resizable(true)
                    .default_width(180.0)
                    .width_range(..=500.0)
                    .show(ctx, |ui| {
                        ui.vertical(|ui| {
                            self.draw_cpu_inspector(ui);
                            ui.separator();
                            self.draw_ppu_inspector(ui);
                        });
                    });
                egui::SidePanel::right("right_panel")
                    .resizable(true)
                    .default_width(180.0)
                    .width_range(..=200.0)
                    .show(ctx, |ui| {
                        ui.vertical(|ui| {
                            self.draw_rom_details(ui);
                            ui.separator();
                            self.show_input(ui);
                            ui.separator();
                            self.draw_fps(ui);
                        });
                    });
                egui::TopBottomPanel::bottom("bottom_panel")
                    .resizable(true)
                    .height_range(..=500.0)
                    .show(ctx, |ui| {
                        ui.horizontal_top(|ui| {
                            self.draw_memory_viewer(ui);
                            if self.args.log {
                                ui.separator();
                                self.draw_log_reader(ui);
                            }
                        });
                    });
            }
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.centered_and_justified(|ui| {
                    self.draw_screen(ctx, ui);
                });
            });
        } else {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.centered_and_justified(|ui| {
                    self.draw_start_screen(ctx, ui);
                });
            });
        }
        self.send_command(Command::Update);
        ctx.request_repaint();
    }

    pub fn handle_emu_events(&mut self, _ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Some(event_rx) = &self.event_rx {
            while let Ok(event) = event_rx.try_recv() {
                match event {
                    Event::Paused => {
                        self.paused = true;
                    }
                    Event::Resumed => {
                        self.paused = false;
                    }
                    Event::Stopped => {
                        self.running = false;
                        self.paused = false;
                    }
                    Event::Crashed(e) => {
                        self.emu_error_msg = Some(e);
                        self.running = false;
                        self.paused = false;
                    }
                    Event::FrameReady(frame_arc) => {
                        self.pixels_buffer = Some(frame_arc);
                        self.frame_ready = true;
                    }
                }
            }
        }
    }

    fn take_snapshot(&self) {
        let frame_data: Vec<u8> = self
            .screen
            .pixels
            .iter()
            .flat_map(|c| {
                let [r, g, b, _a] = c.to_array();
                vec![r, g, b]
            })
            .collect();
        let path = Path::new("screenshot.png");
        match image::save_buffer_with_format(
            path,
            &frame_data,
            self.screen.width as u32,
            self.screen.height as u32,
            image::ColorType::Rgb8,
            image::ImageFormat::Png,
        ) {
            Ok(()) => info!("Image saved to {}", path.display()),
            Err(e) => error!("Couldn't save image: {e}"),
        }
    }
}

impl Drop for Ui {
    fn drop(&mut self) {
        self.stop_emu_thread();
    }
}
