use modular_bitfield::prelude::*;

#[derive(Clone, Debug, Copy, Specifier)]
pub enum SpriteSize {
    _8x8 = 0,
    _8x16 = 1,
}

#[derive(Clone, Debug, Copy, Specifier)]
pub enum PpuMode {
    Master = 0,
    Select = 1,
}

#[bitfield(bytes = 1)]
#[derive(Debug, Clone, Default, Copy)]
pub struct PpuCtrl {
    pub base_nametable_addr: B2,
    pub vram_addr_inc: B1, // VRAM address increment per CPU read/write of PPUDATA
    pub sprite_pattern_table_addr: B1, // for 8x8 sprites; ignored in 8x16 mode
    pub bg_pattern_table_addr: B1,
    pub sprite_size: SpriteSize,
    pub mode: PpuMode,
    pub vblank: bool,
}

impl PpuCtrl {
    pub fn get_base_nametable_addr(&self) -> u16 {
        match self.base_nametable_addr() {
            0 => 0x2000,
            1 => 0x2400,
            2 => 0x2800,
            3 => 0x2C00,
            _ => unreachable!(),
        }
    }

    pub fn get_vram_addr_inc(&self) -> u8 {
        match self.vram_addr_inc() {
            0 => 1,
            1 => 32,
            _ => unreachable!(),
        }
    }

    pub fn get_sprite_pattern_table_addr(&self) -> u16 {
        match self.sprite_pattern_table_addr() {
            0 => 0x0000,
            1 => 0x1000,
            _ => unreachable!(),
        }
    }

    pub fn get_bg_pattern_table_addr(&self) -> u16 {
        match self.bg_pattern_table_addr() {
            0 => 0x0000,
            1 => 0x1000,
            _ => unreachable!(),
        }
    }

    pub fn set(&mut self, value: u8) {
        *self = Self::from_bytes([value]);
    }
}

#[bitfield(bytes = 1)]
#[derive(Debug, Clone, Default)]
pub struct PpuMask {
    pub greyscale: bool,         // (0: normal color, 1: greyscale)
    pub show_background: bool,   // in leftmost 8 pixels of screen, 0: Hide
    pub show_sprites: bool,      // in leftmost 8 pixels of screen, 0: Hide
    pub enable_background: bool, // rendering
    pub enable_sprite: bool,     // rendering
    pub emphasize_red: bool,     // (green on PAL/Dendy)
    pub emphasize_green: bool,   // (red on PAL/Dendy)
    pub emphasize_blue: bool,
}

impl PpuMask {
    pub fn set(&mut self, value: u8) {
        *self = Self::from_bytes([value]);
    }
}

#[bitfield(bytes = 1)]
#[derive(Debug, Clone, Default)]
pub struct PpuStatus {
    pub open_bus: B5, // https://www.nesdev.org/wiki/Open_bus_behavior#PPU_open_bus
    pub sprite_overflow: bool, // https://www.nesdev.org/wiki/PPU_sprite_evaluation#Sprite_overflow_bug
    pub sprite_0_hit: bool,    // https://www.nesdev.org/wiki/PPU_OAM#Sprite_zero_hits
    pub vblank: bool,
}

impl PpuStatus {
    pub fn set(&mut self, value: u8) {
        *self = Self::from_bytes([value]);
    }
}

#[derive(Default, Clone, Debug)]
pub struct OamAddr {
    pub addr: u8,
}

impl OamAddr {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set(&mut self, value: u8) {
        self.addr = value;
    }
}

#[derive(Default, Clone, Debug)]
pub struct OamData {
    pub data: u8,
}

impl OamData {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set(&mut self, value: u8) {
        self.data = value;
    }
}

// https://www.nesdev.org/wiki/PPU_scrolling
#[derive(Default, Clone, Debug)]
pub struct PpuScroll {
    pub x_scroll: u8,
    pub y_scroll: u8,
}

impl PpuScroll {
    pub fn new() -> Self {
        Self {
            x_scroll: 0,
            y_scroll: 0,
        }
    }

    pub fn set(&mut self, value: u8, toggle: &mut bool) {
        if *toggle {
            self.y_scroll = value;
        } else {
            self.x_scroll = value;
        }
        *toggle = !*toggle;
    }
}

#[derive(Default, Clone, Debug)]
pub struct PpuAddr {
    pub addr: u16,
}

impl PpuAddr {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set(&mut self, value: u8, toggle: &mut bool) {
        if *toggle {
            self.addr &= value as u16;
        } else {
            self.addr &= (value as u16) << 8;
        }
        *toggle = !*toggle;
    }
}

#[derive(Default, Clone, Debug)]
pub struct PpuData {
    pub data: u8,
}

impl PpuData {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set(&mut self, value: u8) {
        self.data = value;
    }
}

#[derive(Default, Clone, Debug)]
pub struct OamDma {
    pub dma: u8,
}

impl OamDma {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set(&mut self, value: u8) {
        self.dma = value;
    }
}

#[derive(Default, Clone)]
pub struct Ppu {
    pub scanline: usize,
    pub h_pixel: usize,

    pub ppu_ctrl: PpuCtrl,
    pub ppu_mask: PpuMask,
    pub ppu_status: PpuStatus,
    pub oam_addr: OamAddr,
    pub oam_data: OamData,
    pub ppu_scroll: PpuScroll,
    pub ppu_addr: PpuAddr,
    pub ppu_data: PpuData,
    pub oam_dma: OamDma,

    pub write_toggle: bool,
}

impl Ppu {
    pub fn new() -> Self {
        Self {
            scanline: 0,
            h_pixel: 21, // FIXME?: may be initialized at init state / reset

            ppu_ctrl: PpuCtrl::new(),
            ppu_mask: PpuMask::new(),
            ppu_status: PpuStatus::new(),
            oam_addr: OamAddr::new(),
            oam_data: OamData::new(),
            ppu_scroll: PpuScroll::new(),
            ppu_addr: PpuAddr::new(),
            ppu_data: PpuData::new(),
            oam_dma: OamDma::new(),

            write_toggle: false, // if false we write to first byte / x scroll
        }
    }

    pub fn step(&mut self, cycles: usize) {
        self.h_pixel += 3 * cycles;
        if self.h_pixel > 340 {
            self.scanline += 1;
        }
        self.h_pixel %= 341;
        self.scanline %= 262;
    }

    pub fn check_nmi(&self) -> bool {
        if self.scanline == 241 && self.h_pixel == 1 {
            return self.ppu_ctrl.vblank();
        }
        false
    }
}
