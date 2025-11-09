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
    pub vram_addr_inc: B1,
    pub sprite_pattern_table_addr: B1,
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
    pub greyscale: bool,
    pub show_background: bool,
    pub show_sprites: bool,
    pub enable_background: bool,
    pub enable_sprite: bool,
    pub emphasize_red: bool,
    pub emphasize_green: bool,
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
    pub open_bus: B5,
    pub sprite_overflow: bool,
    pub sprite_0_hit: bool,
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

#[derive(Default, Clone, Debug)]
pub struct PpuScroll {
    pub x_scroll: u8,
    pub y_scroll: u8,
}

impl PpuScroll {
    pub fn new() -> Self {
        Self::default()
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

    pub w: bool,

    pub nmi_output: bool,
    pub nmi_pending: bool,
}

impl Ppu {
    pub fn new() -> Self {
        Self {
            scanline: 0,
            h_pixel: 21,

            ppu_ctrl: PpuCtrl::new(),
            ppu_mask: PpuMask::new(),
            ppu_status: PpuStatus::new(),
            oam_addr: OamAddr::new(),
            oam_data: OamData::new(),
            ppu_scroll: PpuScroll::new(),
            ppu_addr: PpuAddr::new(),
            ppu_data: PpuData::new(),
            oam_dma: OamDma::new(),

            w: false,
            nmi_output: false,
            nmi_pending: false,
        }
    }

    pub fn step(&mut self, cpu_cycles: u8) {
        for _ in 0..cpu_cycles * 3 {
            self.cycle();
        }
    }

    pub fn cycle(&mut self) {
        self.h_pixel += 1;

        if self.h_pixel >= 341 {
            self.h_pixel -= 341;
            self.scanline += 1;

            if self.scanline > 261 {
                self.scanline = 0;
            }
        }

        if self.scanline == 241 && self.h_pixel >= 1 && self.h_pixel < 4 {
            self.ppu_status.set_vblank(true);
            if self.ppu_ctrl.vblank() {
                self.nmi_pending = true;
            }
        }

        if self.scanline == 261 && self.h_pixel >= 1 && self.h_pixel < 4 {
            self.ppu_status.set_vblank(false);
            self.ppu_status.set_sprite_0_hit(false);
            self.ppu_status.set_sprite_overflow(false);
            self.nmi_pending = false;
        }
    }

    pub fn check_nmi(&self) -> bool {
        self.nmi_pending
    }

    pub fn write_ctrl(&mut self, value: u8) {
        self.ppu_ctrl.set(value);
        self.nmi_output = self.ppu_ctrl.vblank();

        if self.ppu_ctrl.vblank() && self.ppu_status.vblank() {
            self.nmi_pending = true;
        }
    }

    pub fn read_status(&mut self) -> u8 {
        let status = self.ppu_status.bytes[0];

        self.ppu_status.set_vblank(false);
        self.w = false;
        self.nmi_pending = false;

        status
    }
}
