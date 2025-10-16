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
#[derive(Debug, Clone)]
struct PpuCtrl {
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
}

#[bitfield(bytes = 1)]
#[derive(Debug, Clone)]
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

#[bitfield(bytes = 1)]
#[derive(Debug, Clone)]
pub struct PpuStatus {
    pub open_bus: B5, // https://www.nesdev.org/wiki/Open_bus_behavior#PPU_open_bus
    pub sprite_overflow: bool, // https://www.nesdev.org/wiki/PPU_sprite_evaluation#Sprite_overflow_bug
    pub sprite_0_hit: bool,    // https://www.nesdev.org/wiki/PPU_OAM#Sprite_zero_hits
    pub vblanki: bool,
}

pub struct OamAddr(u8);

pub struct OamData(u8);

// https://www.nesdev.org/wiki/PPU_scrolling
pub struct PpuScroll(u8);

pub struct PpuAddr(u16);

pub struct PpuData(u8);

pub struct OamDma(u8);

#[derive(Default)]
pub struct Ppu {
    pub scanline: usize,
    pub h_pixel: usize,
}

impl Ppu {
    pub fn new() -> Self {
        Self {
            scanline: 0,
            h_pixel: 21, // FIXME?: may be initialized at init state / reset
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

    // pub fn set_ppu_ctrl (&self,value:u8)  {

    // }

    // pub fn set_ppu_mask (&self,value:u8)  {

    // }

    // pub fn get_ppu_status (&self) -> u8 {

    // }

    // pub fn set_oam_addr(&self,value:u8)  {

    // }

    // pub fn set_oam_data(&self,value:u8)  {

    // }

    // pub fn get_oam_data(&self) -> u8 {

    // }

    // pub fn set_ppu_scroll(&self,value:u8)  {

    // }

    // pub fn set_ppu_addr(&self,value:u8) {

    // }

    // pub fn set_ppu_data(&self,value:u8){

    // }

    // pub fn get_ppu_data(&self) -> u8 {

    // }

    // pub fn set_oam_dma(&self,value:u8) {

    // }
}
