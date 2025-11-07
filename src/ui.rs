use bytesize::ByteSize;
use egui::Color32;
use egui_extras::{Column, TableBuilder};
use log::error;
use rand::prelude::*;
use rfd::FileDialog;
use std::{
    sync::{Arc, mpsc},
    thread::{self, JoinHandle},
};

use crate::{
    args::Args,
    cpu::Flags,
    debug::DebugState,
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
        let mut rng = rand::rng();
        let mut pixels: Vec<Color32> = vec![Color32::BLACK; width * height];
        for i in 0..width {
            for j in 0..height {
                let n = j * width + i;
                pixels[n] =
                    Color32::from_rgb(rng.random::<u8>(), rng.random::<u8>(), rng.random::<u8>());
            }
        }
        Self {
            width,
            height,
            pixels,
            texture_handle: None,
        }
    }

    pub fn update_texture(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        let available = ui.available_size();
        let aspect_ratio = self.width as f32 / self.height as f32;

        let fitted_size = if available.x / available.y > aspect_ratio {
            egui::Vec2::new(available.y * aspect_ratio, available.y)
        } else {
            egui::Vec2::new(available.x, available.x / aspect_ratio)
        };
        let image = egui::ColorImage::new([self.width, self.height], self.pixels.clone());

        let texture: &mut egui::TextureHandle = self.texture_handle.get_or_insert_with(|| {
            ctx.load_texture("screen", image.clone(), egui::TextureOptions::NEAREST)
        });
        ui.image((texture.id(), fitted_size));
    }
}

pub struct Ui {
    screen: Screen,
    command_tx: Option<mpsc::Sender<Command>>,
    event_rx: Option<mpsc::Receiver<Event>>,
    pub debug_state: Arc<DebugState>,
    args: Args,
    emu_thread_handle: Option<JoinHandle<()>>,

    mem_search: String,
    prev_mem_search_addr: usize,

    show_about: bool,

    running: bool,
    paused: bool,
}

impl Ui {
    pub fn new(debug_state: Arc<DebugState>, args: &Args) -> Self {
        Self {
            screen: Screen::new(),
            command_tx: None,
            event_rx: None,
            debug_state,
            args: args.clone(),
            emu_thread_handle: None,
            mem_search: "".into(),
            prev_mem_search_addr: 0,
            show_about: false,
            running: false,
            paused: false,
        }
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

        let handle = thread::Builder::new()
            .name("emu_thread".to_string())
            .spawn(move || {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    emu_thread(command_rx, event_tx, debug_state, &args, &rom);
                }));

                if let Err(e) = result {
                    error!("Emulator thread panicked: {:?}", e);
                    _ = event_tx_clone.send(Event::Crashed);
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
                if ui.button("📥 Select rom...").clicked()
                    && let Some(rom) = FileDialog::new()
                        .add_filter("NES rom", &["nes"])
                        .pick_file()
                {
                    self.spawn_emu_thread(&rom.into_os_string().into_string().unwrap());
                }
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

    fn draw_memory_viewer(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            let n = (match self.mem_search.starts_with("0x") {
                true => usize::from_str_radix(&self.mem_search[2..], 16).unwrap_or(0),
                false => self.mem_search.parse::<usize>().unwrap_or(0),
            })
            .min(0xffff);

            const BYTES_PER_ROW: usize = 0x10;
            const ROWS_TO_SHOW: usize = 7;
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

            ui.label(
                egui::RichText::new(format!("0x{:04X} {}", n, n))
                    .strong()
                    .text_style(egui::TextStyle::Monospace),
            );

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
                                        egui::RichText::new(format!(
                                            "{}",
                                            String::from_iter(bytes_ascii)
                                        ))
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
                        "sp" => format!("{}", cpu.sp + 0x100), format!("0x{:04X}", cpu.sp + 0x100),
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
                egui::ScrollArea::vertical() // FIXME: optimize this
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
            TableBuilder::new(ui)
                .id_salt("ppu")
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                .column(Column::auto())
                .column(Column::remainder())
                .body(|mut body| {
                    make_rows!(body,
                        "H. Pixel" => format!("{}", ppu.h_pixel),
                        "Scanline" => format!("{}", ppu.scanline),
                    );
                });
            egui::ScrollArea::vertical().show(ui, |ui|{
                egui::CollapsingHeader::new("PPU Ctrl").show(ui, |ui| {
                TableBuilder::new(ui)
                    .id_salt("ppuctrl")
                    .striped(true)
                    .column(Column::auto())
                    .column(Column::remainder())
                    .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                    .body(|mut body| {
                        make_rows!(body,
                            "Base nametable addr" => format!("0x{:04X}", ppu.ppu_ctrl.get_base_nametable_addr()),
                            "VRAM addr inc per CPU r/w of PPUDATA" => format!("{}", ppu.ppu_ctrl.get_vram_addr_inc()),
                            "Sprite pattern table addr for 8x8 sprites" => format!("0x{:04X}", ppu.ppu_ctrl.get_sprite_pattern_table_addr()),
                            "Background pattern table address" => format!("0x{:04X}", ppu.ppu_ctrl.get_bg_pattern_table_addr()),
                            "Sprite size" => format!("{:?}", ppu.ppu_ctrl.sprite_size()),
                            "PPU master/slave select" => format!("{:?}", ppu.ppu_ctrl.mode()),
                            "Vblank NMI enable" => format!("{}", ppu.ppu_ctrl.vblank())
                        );
                    });
            });
            egui::CollapsingHeader::new("PPU Mask").show(ui, |ui| {
                TableBuilder::new(ui)
                    .id_salt("ppumask")
                    .striped(true)
                    .column(Column::auto())
                    .column(Column::remainder())
                    .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                    .body(|mut body| {
                        make_rows!(body,
                            "Greyscale (0: normal color, 1: greyscale)" => format!("{}", ppu.ppu_mask.greyscale()),
                            "Show background in leftmost 8 pixels of screen, 0: Hide" => format!("{}", ppu.ppu_mask.show_background()),
                            "Show sprites in leftmost 8 pixels of screen, 0: Hide" => format!("{}", ppu.ppu_mask.show_sprites()),
                            "Enable background rendering" => format!("{}", ppu.ppu_mask.enable_background()),
                            "Enable sprite rendering" => format!("{}", ppu.ppu_mask.enable_sprite()),
                            "Emphasize red (green on PAL/Dendy)" => format!("{}", ppu.ppu_mask.emphasize_red()),
                            "Emphasize green (red on PAL/Dendy)" => format!("{}", ppu.ppu_mask.emphasize_green()),
                            "Emphasize blue" => format!("{}", ppu.ppu_mask.emphasize_blue()),
                        );
                    });
            });
            egui::CollapsingHeader::new("PPU Status").show(ui, |ui| {
                TableBuilder::new(ui)
                    .id_salt("ppustatus")
                    .striped(true)
                    .column(Column::auto())
                    .column(Column::remainder())
                    .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                    .body(|mut body| {
                        make_rows!(body,
                            "PPU open bus or 2C05 PPU identifier" => format!("0x{:02X}", ppu.ppu_status.open_bus()),
                            "Sprite overflow flag" => format!("{}", ppu.ppu_status.sprite_overflow()),
                            "Sprite 0 hit flag" => format!("{}", ppu.ppu_status.sprite_0_hit()),
                            "Vblank flag, cleared on read. Unreliable" => format!("{}", ppu.ppu_status.vblank()),
                        );
                    });
                });
                egui::CollapsingHeader::new("OAM Address").show(ui, |ui| {
                    TableBuilder::new(ui)
                        .id_salt("oamaddr")
                        .striped(true)
                        .column(Column::auto())
                        .column(Column::remainder())
                        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                        .body(|mut body| {
                            make_rows!(body,
                                "OAM Address" => format!("0x{:04X}", ppu.oam_addr.addr),
                            );
                        });
                });
                egui::CollapsingHeader::new("OAM Data").show(ui, |ui| {
                    TableBuilder::new(ui)
                        .id_salt("oamdata")
                        .striped(true)
                        .column(Column::auto())
                        .column(Column::remainder())
                        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                        .body(|mut body| {
                            make_rows!(body,
                                "OAM Data" => format!("{}", ppu.oam_data.data),
                            );
                        });
                });
                egui::CollapsingHeader::new("PPU Scroll").show(ui, |ui| {
                    TableBuilder::new(ui)
                        .id_salt("ppuscroll")
                        .striped(true)
                        .column(Column::auto())
                        .column(Column::remainder())
                        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                        .body(|mut body| {
                            make_rows!(body,
                                "X scroll" => format!("{}", ppu.ppu_scroll.x_scroll),
                                "Y scroll" => format!("{}", ppu.ppu_scroll.y_scroll),
                            );
                        });
                });
                egui::CollapsingHeader::new("PPU Address").show(ui, |ui| {
                    TableBuilder::new(ui)
                        .id_salt("ppuaddr")
                        .striped(true)
                        .column(Column::auto())
                        .column(Column::remainder())
                        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                        .body(|mut body| {
                            make_rows!(body,
                                "PPU address" => format!("0x{:04X}", ppu.ppu_addr.addr),
                            );
                        });
                });
                egui::CollapsingHeader::new("PPU Data").show(ui, |ui| {
                    TableBuilder::new(ui)
                        .id_salt("ppudata")
                        .striped(true)
                        .column(Column::auto())
                        .column(Column::remainder())
                        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                        .body(|mut body| {
                            make_rows!(body,
                                "PPU Data" => format!("{}", ppu.ppu_data.data),
                            );
                        });
                });
                egui::CollapsingHeader::new("OAM Dma").show(ui, |ui| {
                    TableBuilder::new(ui)
                        .id_salt("oamdma")
                        .striped(true)
                        .column(Column::auto())
                        .column(Column::remainder())
                        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                        .body(|mut body| {
                            make_rows!(body,
                                "OAM Dma" => format!("{}", ppu.oam_dma.dma),
                            );
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

    fn draw_log_reader(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            ui.label(egui::RichText::new("Log").strong());
            egui::ScrollArea::vertical()
                .stick_to_bottom(true)
                .auto_shrink(false)
                .show(ui, |ui| {
                    if let Ok(cpu_log) = self.debug_state.cpu_log.read() {
                        let start = cpu_log.len().saturating_sub(3000);
                        let slice = &cpu_log[start..];
                        ui.label(egui::RichText::new(slice).text_style(egui::TextStyle::Monospace));
                    }
                });
        });
    }

    fn draw_screen(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        self.screen.update_texture(ctx, ui);
    }

    fn draw_start_screen(&mut self, _ctx: &egui::Context, ui: &mut egui::Ui) {
        ui.label("Load a rom");
    }

    pub fn draw(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("menubar")
            .resizable(false)
            .show(ctx, |ui| {
                self.draw_menubar(ui);
            });
        if self.running {
            egui::TopBottomPanel::bottom("bottom_panel")
                .resizable(true)
                .default_height(100.0)
                .height_range(..=500.0)
                .show(ctx, |ui| {
                    ui.horizontal_top(|ui| {
                        self.draw_memory_viewer(ui);
                        ui.separator();
                        self.draw_log_reader(ui);
                    });
                });
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
                    });
                });
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
                    Event::Crashed => {
                        self.running = false;
                        self.paused = false;
                    }
                }
            }
        }
    }
}

impl Drop for Ui {
    fn drop(&mut self) {
        self.stop_emu_thread();
    }
}
