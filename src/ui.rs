#[cfg(not(target_arch = "wasm32"))]
use bytesize::ByteSize;
use egui::{Color32, ColorImage, Context, IconData, ImageData, Key, KeyboardShortcut, Modifiers};
#[cfg(not(target_arch = "wasm32"))]
use egui_extras::{Column, TableBuilder};
#[cfg(not(target_arch = "wasm32"))]
use egui_plot::{Line, Plot, PlotPoints};
#[cfg(not(target_arch = "wasm32"))]
use log::{error, info};
#[cfg(not(target_arch = "wasm32"))]
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[cfg(not(target_arch = "wasm32"))]
use std::time::{Duration, Instant};
#[cfg(target_arch = "wasm32")]
use web_time::{Duration, Instant};

#[cfg(not(target_arch = "wasm32"))]
use crate::args::get_args;
use crate::platform::RomSource;
use crate::ppu::{FRAME_HEIGHT, FRAME_WIDTH};
#[cfg(not(target_arch = "wasm32"))]
use crate::{
    debug::{BYTES_PER_ROW, DebugSnapshot, ROWS_TO_SHOW},
    emu::{Command, Event},
    mapper::MapperIcon,
    platform::PlatformRunner,
    ppu::Ppu,
};
#[cfg(target_arch = "wasm32")]
use crate::{
    emu::{Command, Event},
    platform::PlatformRunner,
};

#[cfg(not(target_arch = "wasm32"))]
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

#[repr(usize)]
#[derive(Debug, Clone, Copy, PartialEq)]
enum AppAction {
    PauseResume = 0,
    Step,
    #[cfg(not(target_arch = "wasm32"))]
    SaveState,
    #[cfg(not(target_arch = "wasm32"))]
    TakeScreenshot,
    OpenRom,
    Quit,
    // Reset,
    // ToggleDebug,
}

impl AppAction {
    #[inline(always)]
    pub fn shortcut(&self) -> KeyboardShortcut {
        DEFAULT_SHORTCUTS[*self as usize].1
    }
}

const DEFAULT_SHORTCUTS: &[(AppAction, KeyboardShortcut)] = &[
    (
        AppAction::PauseResume,
        KeyboardShortcut {
            modifiers: Modifiers::NONE,
            logical_key: Key::Space,
        },
    ),
    (
        AppAction::Step,
        KeyboardShortcut {
            modifiers: Modifiers::NONE,
            logical_key: Key::Enter,
        },
    ),
    #[cfg(not(target_arch = "wasm32"))]
    (
        AppAction::SaveState,
        KeyboardShortcut {
            modifiers: Modifiers::NONE,
            logical_key: Key::F5,
        },
    ),
    #[cfg(not(target_arch = "wasm32"))]
    (
        AppAction::TakeScreenshot,
        KeyboardShortcut {
            modifiers: Modifiers::NONE,
            logical_key: Key::F12,
        },
    ),
    (
        AppAction::OpenRom,
        KeyboardShortcut {
            modifiers: Modifiers::CTRL,
            logical_key: Key::O,
        },
    ),
    (
        AppAction::Quit,
        KeyboardShortcut {
            modifiers: Modifiers::CTRL,
            logical_key: Key::Q,
        },
    ),
];

#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub struct ControllerState {
    pub a: bool,
    pub b: bool,
    pub select: bool,
    pub start: bool,
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
}

impl ControllerState {
    pub fn to_u8(&self) -> u8 {
        let mut byte = 0;
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

#[derive(Default)]
struct InputManager;

impl InputManager {
    fn update(&self, ctx: &egui::Context) -> (Vec<AppAction>, ControllerState) {
        let mut triggered_actions = Vec::new();
        let mut controller = ControllerState::default();

        if ctx.wants_keyboard_input() {
            return (triggered_actions, controller);
        }

        ctx.input_mut(|i| {
            for (action, shortcut) in DEFAULT_SHORTCUTS {
                if i.consume_shortcut(shortcut) {
                    triggered_actions.push(*action);
                }
            }

            controller.a = i.key_down(Key::A);
            controller.b = i.key_down(Key::B);
            controller.start = i.key_down(Key::Z);
            controller.select = i.key_down(Key::N);
            controller.up = i.key_down(Key::ArrowUp);
            controller.down = i.key_down(Key::ArrowDown);
            controller.left = i.key_down(Key::ArrowLeft);
            controller.right = i.key_down(Key::ArrowRight);
        });

        (triggered_actions, controller)
    }
}

pub struct FrameStats {
    fps: f32,
    fps_last_update: Instant,
    render_count_since_fps_update: u64,
}

impl Default for FrameStats {
    fn default() -> Self {
        Self {
            fps: Default::default(),
            fps_last_update: Instant::now(),
            render_count_since_fps_update: Default::default(),
        }
    }
}

impl FrameStats {
    pub fn new() -> Self {
        Self {
            fps: 0.0,
            fps_last_update: Instant::now(),
            render_count_since_fps_update: 0,
        }
    }

    fn update_fps(&mut self) {
        let elapsed_duration = self.fps_last_update.elapsed();
        let one_second = Duration::from_secs(1);

        if elapsed_duration >= one_second {
            let elapsed_secs = elapsed_duration.as_secs_f32();

            self.fps = self.render_count_since_fps_update as f32 / elapsed_secs;

            self.render_count_since_fps_update = 0;
            self.fps_last_update = Instant::now();
        }
    }

    pub fn tick(&mut self) {
        self.render_count_since_fps_update += 1;
        self.update_fps();
    }
}

#[derive(Default)]
pub struct Screen {
    pub width: usize,
    pub height: usize,
    pub texture_handle: Option<egui::TextureHandle>,
}

impl Screen {
    pub fn new() -> Self {
        Self {
            width: FRAME_WIDTH,
            height: FRAME_HEIGHT,
            texture_handle: None,
        }
    }

    pub fn update_texture(&mut self, ctx: &egui::Context, ui: &mut egui::Ui, pixels: &[Color32]) {
        let image = egui::ColorImage::new([self.width, self.height], pixels.to_owned());

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
    runner: PlatformRunner,
    app_icon_texture: egui::TextureHandle,

    #[cfg(not(target_arch = "wasm32"))]
    snapshot: DebugSnapshot,

    input_manager: InputManager,
    last_controller_input: u16,

    emu_error_msg: Option<String>,
    #[cfg(not(target_arch = "wasm32"))]
    log: bool,

    #[cfg(not(target_arch = "wasm32"))]
    mem_search: String,
    #[cfg(not(target_arch = "wasm32"))]
    prev_mem_search_addr: usize,

    show_about: bool,
    #[cfg(not(target_arch = "wasm32"))]
    show_debug_panels: bool,

    running: bool,
    paused: bool,
    frame_stats: FrameStats,
}

impl Ui {
    pub fn new(ctx: &egui::Context) -> Self {
        let app_icon = Self::app_icon();
        let app_icon_img = ImageData::Color(Arc::new(ColorImage::from_rgba_unmultiplied(
            [app_icon.width as usize, app_icon.height as usize],
            &app_icon.rgba,
        )));
        let app_icon_texture =
            ctx.load_texture("app_icon", app_icon_img, egui::TextureOptions::NEAREST);
        Self {
            screen: Screen::new(),
            runner: Default::default(),
            app_icon_texture,

            #[cfg(not(target_arch = "wasm32"))]
            snapshot: Default::default(),

            input_manager: Default::default(),
            last_controller_input: 0,

            emu_error_msg: None,
            #[cfg(not(target_arch = "wasm32"))]
            log: get_args().log,

            #[cfg(not(target_arch = "wasm32"))]
            mem_search: "".into(),
            #[cfg(not(target_arch = "wasm32"))]
            prev_mem_search_addr: 0,

            show_about: false,
            #[cfg(all(not(target_arch = "wasm32"), debug_assertions))]
            show_debug_panels: true,
            #[cfg(all(not(target_arch = "wasm32"), not(debug_assertions)))]
            show_debug_panels: false,

            running: false,
            paused: false,
            frame_stats: FrameStats::new(),
        }
    }

    pub fn handle_input(&mut self, ctx: &egui::Context) {
        let (actions, controller) = self.input_manager.update(ctx);

        let input_val = controller.to_u8() as u16;

        if input_val != self.last_controller_input {
            self.runner
                .send_command(Command::ControllerInputs(input_val));
            self.last_controller_input = input_val;
        }

        for action in actions {
            self.dispatch_action(ctx, action);
        }
    }

    fn dispatch_action(&mut self, ctx: &Context, action: AppAction) {
        match action {
            AppAction::PauseResume => {
                if self.paused {
                    self.runner.resume();
                } else {
                    self.runner.pause();
                }
            }
            AppAction::Step => {
                if self.paused {
                    self.runner.step();
                }
            }
            #[cfg(not(target_arch = "wasm32"))]
            AppAction::SaveState => {
                self.runner.send_command(Command::SaveState);
            }
            #[cfg(not(target_arch = "wasm32"))]
            AppAction::TakeScreenshot => {
                self.take_screenshot();
            }
            AppAction::OpenRom => {
                self.open_rom();
            }
            AppAction::Quit => {
                self.quit(ctx);
            }
        }
    }

    fn quit(&self, ctx: &Context) {
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }

    fn stop(&mut self) {
        self.runner.stop();
        self.running = false;
        self.paused = false;
    }

    pub fn start(&mut self, rom: RomSource) {
        if self.running {
            self.stop();
        }
        self.runner.start(rom);
        self.running = true;
        self.paused = false;
    }

    pub fn app_icon() -> IconData {
        let icon_data = include_bytes!("../assets/icon-1024.png");
        let icon = image::load_from_memory(icon_data)
            .expect("Failed to load application icon")
            .to_rgba8();
        let (width, height) = icon.dimensions();

        IconData {
            rgba: icon.into_raw(),
            width,
            height,
        }
    }

    fn open_rom(&mut self) {
        if let Some(rom) = self.runner.pick_rom() {
            self.start(RomSource::Path(rom));
        }
    }

    fn draw_menubar(&mut self, ui: &mut egui::Ui) {
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui
                    .add(
                        egui::Button::new("🎮 Open ROM...").shortcut_text(
                            ui.ctx().format_shortcut(&AppAction::OpenRom.shortcut()),
                        ),
                    )
                    .clicked()
                {
                    self.open_rom();
                }

                #[cfg(not(target_arch = "wasm32"))]
                {
                    ui.separator();
                    ui.add_enabled_ui(self.running, |ui| {
                        if ui
                            .add(egui::Button::new("📥 Save state").shortcut_text(
                                ui.ctx().format_shortcut(&AppAction::SaveState.shortcut()),
                            ))
                            .clicked()
                        {
                            self.runner.send_command(Command::SaveState);
                        }
                        if ui.add(egui::Button::new("📥 Load state")).clicked() {
                            self.runner.pick_state_file();
                        }
                    });
                    ui.separator();
                    if ui
                        .add(
                            egui::Button::new("✖ Quit").shortcut_text(
                                ui.ctx().format_shortcut(&AppAction::Quit.shortcut()),
                            ),
                        )
                        .clicked()
                    {
                        self.quit(ui.ctx());
                    }
                }
            });
            ui.menu_button("Emulator", |ui| {
                ui.add_enabled_ui(self.running && self.paused, |ui| {
                    if ui
                        .add(
                            egui::Button::new("⤵ Step").shortcut_text(
                                ui.ctx().format_shortcut(&AppAction::Step.shortcut()),
                            ),
                        )
                        .clicked()
                    {
                        self.runner.step();
                    }
                });
                ui.add_enabled_ui(self.running && self.paused, |ui| {
                    if ui
                        .add(egui::Button::new("▶ Resume").shortcut_text(
                            ui.ctx().format_shortcut(&AppAction::PauseResume.shortcut()),
                        ))
                        .clicked()
                    {
                        self.runner.resume();
                    }
                });
                ui.add_enabled_ui(self.running && !self.paused, |ui| {
                    if ui
                        .add(egui::Button::new("⏸ Pause").shortcut_text(
                            ui.ctx().format_shortcut(&AppAction::PauseResume.shortcut()),
                        ))
                        .clicked()
                    {
                        self.runner.pause();
                    }
                });
                ui.add_enabled_ui(self.running, |ui| {
                    if ui.button("⏹ Stop").clicked() {
                        self.stop();
                    }
                });
                #[cfg(not(target_arch = "wasm32"))]
                {
                    ui.separator();
                    ui.add_enabled_ui(self.running, |ui| {
                        if ui
                            .add(
                                egui::Button::new("📷 Take screenshot").shortcut_text(
                                    ui.ctx()
                                        .format_shortcut(&AppAction::TakeScreenshot.shortcut()),
                                ),
                            )
                            .clicked()
                        {
                            self.take_screenshot();
                        }
                    });
                    ui.checkbox(&mut self.show_debug_panels, "Show debug panels");
                }
            });
            ui.menu_button("Help", |ui| {
                if ui.button("ℹ About").clicked() {
                    self.show_about = true;
                }
            });
            if self.show_about {
                let modal = egui::Modal::new(egui::Id::new("about_modal")).show(ui.ctx(), |ui| {
                    ui.set_width(600.0);
                    ui.horizontal(|ui| {
                        ui.image((self.app_icon_texture.id(), [280.0, 280.0].into()));
                        ui.add_space(16.0);
                        ui.vertical(|ui| {
                            ui.add_space(16.0);
                            ui.label(egui::RichText::new("Nessie").strong().size(36.0));
                            ui.separator();
                            ui.add_space(16.0);

                            let platform = if cfg!(target_arch = "wasm32") {
                                "Web"
                            } else {
                                std::env::consts::OS
                            };

                            let arch = std::env::consts::ARCH;

                            ui.label(egui::RichText::new("Nintendo NES emulator").size(22.0));
                            ui.add_space(8.0);
                            ui.label(format!("Platform: {} ({})", platform, arch));

                            let date = format!(
                                "{} {}",
                                compile_time::date_str!(),
                                compile_time::time_str!()
                            );

                            ui.label(format!("Date: {date}"));
                            ui.label(format!(
                                "Version: {}",
                                option_env!("VERSION").unwrap_or("unknown")
                            ));
                            ui.add_space(8.0);

                            ui.add(
                                egui::Hyperlink::from_label_and_url(
                                    " Nessie on GitHub",
                                    "https://github.com/mdmrk/nessie",
                                )
                                .open_in_new_tab(true),
                            );
                        });
                    });
                });
                if modal.should_close() {
                    self.show_about = false;
                }
            }
        });
    }

    #[cfg(not(target_arch = "wasm32"))]
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
                self.runner.send_command(Command::MemoryAddress(start_addr));
                self.prev_mem_search_addr = start_addr;
            }

            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(format!("0x{:04X} {}", n, n))
                        .strong()
                        .text_style(egui::TextStyle::Monospace),
                );
                if ui.button("Dump").clicked() {
                    self.runner.send_command(Command::DumpMemory);
                }
            });

            TableBuilder::new(ui)
                .striped(true)
                .column(Column::auto())
                .column(Column::auto())
                .column(Column::auto())
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                .body(|mut body| {
                    for (i, lines) in self
                        .snapshot
                        .mem_chunk
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
                                    egui::RichText::new(String::from_iter(bytes_ascii).to_string())
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
        });
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn draw_cpu_inspector(&mut self, ui: &mut egui::Ui) {
        let cpu = &self.snapshot.cpu;
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
                );
            });

        let flags = [
            (cpu.flags_n, "N"),
            (cpu.flags_v, "V"),
            (cpu.flags_b, "B"),
            (cpu.flags_d, "D"),
            (cpu.flags_i, "I"),
            (cpu.flags_z, "Z"),
            (cpu.flags_c, "C"),
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
                        for (mut val, label) in flags {
                            row.col(|ui| {
                                ui.add_enabled(false, egui::Checkbox::new(&mut val, label));
                            });
                        }
                    })
                });
        });

        egui::CollapsingHeader::new("Stack").show(ui, |ui| {
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
                            for (i, n) in self.snapshot.stack.iter().enumerate().rev() {
                                make_rows!(body,
                                    format!("0x{:04X}", i + 0x100) =>
                                        format!("{}", n),
                                        format!("0x{:02X}",n));
                            }
                        });
                });
        });
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn draw_apu_inspector(&self, ui: &mut egui::Ui) {
        ui.label(egui::RichText::new("APU").strong());
        let apu = &self.snapshot.apu;
        egui::CollapsingHeader::new("Pulse 1 & 2").show(ui, |ui| {
            TableBuilder::new(ui)
                .id_salt("apu_pulse")
                .striped(true)
                .column(Column::auto())
                .column(Column::remainder())
                .body(|mut body| {
                    let hz1 = if apu.pulse1_period > 0 {
                        1789773.0 / (16.0 * (apu.pulse1_period as f32 + 1.0))
                    } else {
                        0.0
                    };
                    let hz2 = if apu.pulse2_period > 0 {
                        1789773.0 / (16.0 * (apu.pulse2_period as f32 + 1.0))
                    } else {
                        0.0
                    };

                    make_rows!(body,
                        "P1 Enable" => format!("{}", apu.pulse1_enabled),
                        "P1 Freq" => format!("{:.1} Hz", hz1),
                        "P1 Vol" => format!("{}", apu.pulse1_vol),
                        "P1 Duty" => format!("{}", apu.pulse1_duty),
                        "P2 Enable" => format!("{}", apu.pulse2_enabled),
                        "P2 Freq" => format!("{:.1} Hz", hz2),
                        "P2 Vol" => format!("{}", apu.pulse2_vol),
                    );
                });
        });

        egui::CollapsingHeader::new("Triangle & Noise").show(ui, |ui| {
            TableBuilder::new(ui)
                .id_salt("apu_t_n")
                .striped(true)
                .column(Column::auto())
                .column(Column::remainder())
                .body(|mut body| {
                    make_rows!(body,
                        "Tri Enable" => format!("{}", apu.tri_enabled),
                        "Tri Linear" => format!("{}", apu.tri_linear),
                        "Noise Enable" => format!("{}", apu.noise_enabled),
                        "Noise Mode" => format!("{}", apu.noise_mode),
                    );
                });
        });

        egui::CollapsingHeader::new("DMC & Frame").show(ui, |ui| {
            TableBuilder::new(ui)
                .id_salt("apu_dmc")
                .striped(true)
                .column(Column::auto())
                .column(Column::remainder())
                .body(|mut body| {
                    make_rows!(body,
                        "DMC Enable" => format!("{}", apu.dmc_enabled),
                        "DMC Bytes" => format!("{}", apu.dmc_len),
                        "Frame Mode" => if apu.frame_mode { "5-Step" } else { "4-Step" },
                        "IRQ Pending" => format!("{}", apu.frame_irq || apu.dmc_irq),
                    );
                });
        });
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn draw_ppu_inspector(&mut self, ui: &mut egui::Ui) {
        let ppu = &self.snapshot.ppu;
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
                            let nmi_enable = ppu.ctrl.nmi_enable();

                            make_rows!(body,
                                "Base nametable" => format!("0x{:04X}", base_nt_addr),
                                "VRAM addr increment" => format!("{}", vram_inc),
                                "Sprite pattern table" => format!("0x{:04X}", sprite_pt_addr),
                                "BG pattern table" => format!("0x{:04X}", bg_pt_addr),
                                "Sprite size" => format!("{}", sprite_size),
                                "Master/slave" => format!("{}", master_slave),
                                "NMI enable" => format!("{}", nmi_enable)
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
                                                    egui::RichText::new(format!("Sprite {}", i))
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
                                    for (row_idx, row_colors) in ppu.palette.chunks(8).enumerate() {
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

                                                let color = Ppu::get_color_from_palette(
                                                    displayed_idx & 0x3F,
                                                );

                                                let mut text = egui::RichText::new(format!(
                                                    "{:02X}",
                                                    color_idx
                                                ))
                                                .text_style(egui::TextStyle::Monospace);
                                                if [0x10, 0x14, 0x18, 0x1C].contains(&palette_addr)
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
                                                let color = Ppu::get_color_from_palette(
                                                    displayed_idx & 0x3F,
                                                );

                                                let mut text = egui::RichText::new(format!(
                                                    "{:02X}",
                                                    color_idx
                                                ))
                                                .text_style(egui::TextStyle::Monospace);
                                                if [0x10, 0x14, 0x18, 0x1C].contains(&palette_addr)
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

    #[cfg(not(target_arch = "wasm32"))]
    fn draw_sound_waves(&mut self, ui: &mut egui::Ui) {
        let apu = &self.snapshot.apu;
        ui.label(egui::RichText::new("Pulse 1 & 2").strong());
        Plot::new("pulses").view_aspect(2.0).show(ui, |plot_ui| {
            let hz1 = if apu.pulse1_period > 0 {
                1789773.0 / (16.0 * (apu.pulse1_period as f32 + 1.0))
            } else {
                0.0
            };
            let hz2 = if apu.pulse2_period > 0 {
                1789773.0 / (16.0 * (apu.pulse2_period as f32 + 1.0))
            } else {
                0.0
            };
            let points1 = generate_pulse_wave(hz1 as f64, 1.0, 0.5, 0.016);
            let points2 = generate_pulse_wave(hz2 as f64, 1.0, 0.5, 0.016);
            let line1 = Line::new("pulse1", PlotPoints::new(points1)).width(2.0);
            let line2 = Line::new("pulse2", PlotPoints::new(points2)).width(2.0);
            plot_ui.line(line1);
            plot_ui.line(line2);
        });
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn draw_palette_colors(&self, ui: &mut egui::Ui) {
        let ppu = &self.snapshot.ppu;
        ui.label(egui::RichText::new("Palette Colors").strong());

        egui::ScrollArea::vertical()
            .id_salt("palette")
            .show(ui, |ui| {
                let square_size = 20.0;
                ui.label("Background Palettes");
                for palette_idx in 0..4 {
                    ui.horizontal(|ui| {
                        ui.label(format!("BG{}: ", palette_idx));
                        for color_idx in 0..4 {
                            let addr = palette_idx * 4 + color_idx;
                            let color_byte = ppu.palette[addr];
                            let color = Ppu::get_color_from_palette(color_byte & 0x3F);

                            let (rect, _response) = ui.allocate_exact_size(
                                egui::vec2(square_size, square_size),
                                egui::Sense::hover(),
                            );
                            ui.painter().rect_filled(rect, 0.0, color);
                        }
                    });
                }

                ui.add_space(8.0);

                ui.label("Sprite Palettes");
                for palette_idx in 0..4 {
                    ui.horizontal(|ui| {
                        ui.label(format!("SP{}: ", palette_idx));
                        for color_idx in 0..4 {
                            let addr = 0x10 + palette_idx * 4 + color_idx;
                            let color_byte = match addr {
                                0x10 | 0x14 | 0x18 | 0x1C => ppu.palette[addr - 0x10],
                                _ => ppu.palette[addr],
                            };
                            let color = Ppu::get_color_from_palette(color_byte & 0x3F);

                            let (rect, _response) = ui.allocate_exact_size(
                                egui::vec2(square_size, square_size),
                                egui::Sense::hover(),
                            );
                            ui.painter().rect_filled(rect, 0.0, color);
                        }
                    });
                }
            });
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn draw_rom_details(&mut self, ui: &mut egui::Ui) {
        ui.label(egui::RichText::new("ROM details").strong());
        match &self.snapshot.cart {
            Some(cart) => {
                TableBuilder::new(ui)
                    .striped(true)
                    .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                    .column(Column::auto())
                    .column(Column::remainder())
                    .body(|mut body| {
                        make_rows!(body,
                            "ROM hash" => format!("{}...", &cart.hash[0..8]),
                            "PRG ROM Size" =>
                                format!("{}", ByteSize::kib(16) * (cart.prg_rom_size as u64)),
                            "CHR ROM Size" =>
                                format!("{}", ByteSize::kib(8) * (cart.chr_rom_size as u64)),
                        );
                        body.row(16.0, |mut row| {
                            row.col(|ui| {
                                ui.label(egui::RichText::new("Mapper").strong());
                            });
                            row.col(|ui| {
                                let mapper_num = cart.mapper_number;

                                let icon = egui::Image::from_bytes(
                                    format!("mapper_icon_{}", mapper_num),
                                    MapperIcon::from_mapper_number(mapper_num).bytes(),
                                );

                                ui.add(icon);
                                ui.label(format!("{}", mapper_num));
                            });
                        });
                    });
            }
            None => {
                ui.label("Not loaded");
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn draw_fps(&self, ui: &mut egui::Ui) {
        ui.label(format!("FPS: {:.1}", self.frame_stats.fps));
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn draw_log_reader(&self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            ui.label(egui::RichText::new("Log").strong());
            egui::ScrollArea::vertical()
                .stick_to_bottom(true)
                .auto_shrink(false)
                .show(ui, |ui| {
                    if let Some(log) = &self.snapshot.cpu.log {
                        ui.label(egui::RichText::new(log).text_style(egui::TextStyle::Monospace));
                    }
                });
        });
    }

    fn draw_screen(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        if let Some(pixels) = self.runner.get_frame_data() {
            self.screen.update_texture(ctx, ui, pixels);
            self.frame_stats.tick();
        }
    }

    fn draw_start_screen(&self, _ctx: &egui::Context, ui: &mut egui::Ui) {
        match &self.emu_error_msg {
            Some(msg) => ui.colored_label(Color32::RED, msg),
            None => ui.label("Load a rom"),
        };
    }

    pub fn draw(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(snapshot) = self.runner.get_debug_snapshot() {
            self.snapshot = snapshot;
        }

        egui::TopBottomPanel::top("menubar")
            .resizable(false)
            .show(ctx, |ui| {
                self.draw_menubar(ui);
            });
        if self.running {
            #[cfg(not(target_arch = "wasm32"))]
            if self.show_debug_panels {
                egui::SidePanel::left("left_panel")
                    .resizable(true)
                    .default_width(180.0)
                    .width_range(100.0..=500.0)
                    .show(ctx, |ui| {
                        ui.columns_const(|[col_1, col_2]| {
                            col_1.vertical(|ui| {
                                self.draw_cpu_inspector(ui);
                                ui.separator();
                                self.draw_apu_inspector(ui);
                                ui.separator();
                                self.draw_ppu_inspector(ui);
                            });
                            col_2.vertical(|ui| {
                                self.draw_palette_colors(ui);
                                ui.separator();
                                self.draw_sound_waves(ui);
                            });
                        });
                    });
                egui::SidePanel::right("right_panel")
                    .resizable(true)
                    .default_width(190.0)
                    .width_range(..=200.0)
                    .show(ctx, |ui| {
                        ui.vertical(|ui| {
                            self.draw_rom_details(ui);
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
                            if self.log {
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
        ctx.request_repaint();
    }

    pub fn handle_emu_events(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let events = self.runner.handle_events(ctx);
        for event in events {
            match event {
                Event::Started => {
                    self.running = true;
                    self.paused = false;
                    self.emu_error_msg = None;
                }
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
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn take_screenshot(&mut self) {
        let frame_data: Vec<u8> = self
            .runner
            .frame_rx
            .as_mut()
            .unwrap()
            .read()
            .iter()
            .flat_map(|c| {
                let [r, g, b, _a] = c.to_array();
                vec![r, g, b]
            })
            .collect();
        let path = get_unique_path();
        match image::save_buffer_with_format(
            path.clone(),
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
        self.stop();
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn get_unique_path() -> PathBuf {
    let directory = ".";
    let filename = "screenshot";
    let extension = "png";
    let mut count = 0;
    let mut path = Path::new(directory).join(format!("{}.{}", filename, extension));

    while path.exists() {
        count += 1;
        let new_filename = format!("{}_{}.{}", filename, count, extension);
        path = Path::new(directory).join(new_filename);
    }

    path
}

#[cfg(not(target_arch = "wasm32"))]
fn generate_pulse_wave(freq: f64, amp: f64, duty_cycle: f64, duration: f64) -> Vec<[f64; 2]> {
    let mut points = Vec::new();
    let period = 1.0 / freq;

    let high_duration = period * duty_cycle;
    let low_duration = period - high_duration;

    let mut current_time = 0.0;

    while current_time < duration {
        let high_start = current_time;
        let high_end = (high_start + high_duration).min(duration);

        points.push([high_start, amp]);
        points.push([high_end, amp]);

        current_time = high_end;
        if current_time >= duration {
            break;
        }
        let low_start = current_time;
        let low_end = (low_start + low_duration).min(duration);
        points.push([low_start, 0.0]);
        points.push([low_end, 0.0]);

        current_time = low_end;
    }
    points
}
