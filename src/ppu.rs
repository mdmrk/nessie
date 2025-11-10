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
#[derive(Debug, Clone, Default, Copy)]
pub struct PpuMask {
    pub greyscale: bool,
    pub show_background_left: bool,
    pub show_sprites_left: bool,
    pub show_background: bool,
    pub show_sprites: bool,
    pub emphasize_red: bool,
    pub emphasize_green: bool,
    pub emphasize_blue: bool,
}

impl PpuMask {
    pub fn set(&mut self, value: u8) {
        *self = Self::from_bytes([value]);
    }

    pub fn rendering_enabled(&self) -> bool {
        self.show_background() || self.show_sprites()
    }
}

#[bitfield(bytes = 1)]
#[derive(Debug, Clone, Default, Copy)]
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

    pub fn get(&self) -> u8 {
        self.bytes[0]
    }
}

#[derive(Clone, Debug)]
pub struct Ppu {
    pub scanline: usize,
    pub dot: usize,
    pub frame: u64,
    pub frame_ready: bool,

    pub ppu_ctrl: PpuCtrl,
    pub ppu_mask: PpuMask,
    pub ppu_status: PpuStatus,

    pub oam: [u8; 256],
    pub oam_addr: u8,

    pub secondary_oam: [u8; 32],
    pub sprite_count: usize,

    pub vram: [u8; 2048],
    pub palette: [u8; 32],

    pub v: u16,
    pub t: u16,
    pub x: u8,
    pub w: bool,

    pub buffer: u8,
    pub bus: u8,

    pub nmi_pending: bool,
    pub nmi_delay: u8,

    pub bg_shift_lo: u16,
    pub bg_shift_hi: u16,
    pub bg_attr_shift_lo: u16,
    pub bg_attr_shift_hi: u16,

    pub bg_next_tile_id: u8,
    pub bg_next_tile_attr: u8,
    pub bg_next_tile_lo: u8,
    pub bg_next_tile_hi: u8,

    pub sprite_patterns_lo: [u8; 8],
    pub sprite_patterns_hi: [u8; 8],
    pub sprite_positions: [u8; 8],
    pub sprite_priorities: [u8; 8],
    pub sprite_indices: [u8; 8],

    pub screen: Vec<u32>,
}

impl Default for Ppu {
    fn default() -> Self {
        Self {
            scanline: Default::default(),
            dot: 21,
            frame: Default::default(),
            frame_ready: false,
            ppu_ctrl: Default::default(),
            ppu_mask: Default::default(),
            ppu_status: Default::default(),
            oam: [0; 256],
            oam_addr: Default::default(),
            secondary_oam: Default::default(),
            sprite_count: Default::default(),
            vram: [0; 2048],
            palette: Default::default(),
            v: Default::default(),
            t: Default::default(),
            x: Default::default(),
            w: Default::default(),
            buffer: Default::default(),
            bus: Default::default(),
            nmi_pending: Default::default(),
            nmi_delay: Default::default(),
            bg_shift_lo: Default::default(),
            bg_shift_hi: Default::default(),
            bg_attr_shift_lo: Default::default(),
            bg_attr_shift_hi: Default::default(),
            bg_next_tile_id: Default::default(),
            bg_next_tile_attr: Default::default(),
            bg_next_tile_lo: Default::default(),
            bg_next_tile_hi: Default::default(),
            sprite_patterns_lo: Default::default(),
            sprite_patterns_hi: Default::default(),
            sprite_positions: Default::default(),
            sprite_priorities: Default::default(),
            sprite_indices: Default::default(),
            screen: vec![0; 256 * 240],
        }
    }
}

impl Ppu {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) {
        self.ppu_ctrl = PpuCtrl::new();
        self.ppu_mask = PpuMask::new();
        self.w = false;
        self.buffer = 0;
        self.scanline = 0;
        self.dot = 0;
    }

    pub fn step(&mut self, mapper: &mut dyn crate::mapper::Mapper, cycles: u8) {
        for _ in 0..cycles * 3 {
            self.cycle(mapper);
        }
    }

    pub fn cycle(&mut self, mapper: &mut dyn crate::mapper::Mapper) {
        if self.scanline < 240 {
            self.render_scanline(mapper);
        } else if self.scanline == 241 && self.dot == 1 {
            self.ppu_status.set_vblank(true);
            if self.ppu_ctrl.vblank() {
                self.nmi_pending = true;
            }
        } else if self.scanline == 261 {
            self.prerender_scanline(mapper);
        }

        self.dot += 1;

        if self.dot > 340 {
            self.dot = 0;
            self.scanline += 1;

            if self.scanline > 261 {
                self.scanline = 0;
                self.frame += 1;
                self.frame_ready = true;
            }
        }
    }

    fn render_scanline(&mut self, mapper: &mut dyn crate::mapper::Mapper) {
        if self.dot == 0 {
            return;
        }

        if self.ppu_mask.rendering_enabled() {
            if self.dot >= 1 && self.dot <= 256 {
                self.update_shifters();

                match (self.dot - 1) % 8 {
                    0 => {
                        self.load_background_shifters();
                        self.fetch_nametable_byte(mapper);
                    }
                    2 => self.fetch_attribute_byte(mapper),
                    4 => self.fetch_pattern_low_byte(mapper),
                    6 => self.fetch_pattern_high_byte(mapper),
                    7 => self.increment_scroll_x(),
                    _ => {}
                }
            }

            if self.dot == 256 {
                self.increment_scroll_y();
            }

            if self.dot == 257 {
                self.load_background_shifters();
                self.copy_horizontal_position();
            }

            if self.dot == 338 || self.dot == 340 {
                self.fetch_nametable_byte(mapper);
            }

            if self.dot == 257 && self.scanline < 240 {
                self.evaluate_sprites();
            }

            if self.dot == 340 {
                self.load_sprite_data(mapper);
            }
        }

        if self.dot >= 1 && self.dot <= 256 && self.scanline < 240 {
            self.render_pixel();
        }
    }

    fn prerender_scanline(&mut self, mapper: &mut dyn crate::mapper::Mapper) {
        if self.dot == 1 {
            self.ppu_status.set_vblank(false);
            self.ppu_status.set_sprite_0_hit(false);
            self.ppu_status.set_sprite_overflow(false);
            self.nmi_pending = false;
        }

        if self.ppu_mask.rendering_enabled() {
            if self.dot >= 280 && self.dot <= 304 {
                self.copy_vertical_position();
            }

            if self.dot >= 1 && self.dot <= 256 {
                self.update_shifters();

                match (self.dot - 1) % 8 {
                    0 => {
                        self.load_background_shifters();
                        self.fetch_nametable_byte(mapper);
                    }
                    2 => self.fetch_attribute_byte(mapper),
                    4 => self.fetch_pattern_low_byte(mapper),
                    6 => self.fetch_pattern_high_byte(mapper),
                    7 => self.increment_scroll_x(),
                    _ => {}
                }
            }

            if self.dot == 256 {
                self.increment_scroll_y();
            }

            if self.dot == 257 {
                self.load_background_shifters();
                self.copy_horizontal_position();
            }

            if self.dot == 338 || self.dot == 340 {
                self.fetch_nametable_byte(mapper);
            }
        }

        if self.dot == 340 && self.frame % 2 == 1 && self.ppu_mask.rendering_enabled() {
            self.dot = 0;
            self.scanline = 0;
        }
    }

    fn fetch_nametable_byte(&mut self, mapper: &mut dyn crate::mapper::Mapper) {
        let addr = 0x2000 | (self.v & 0x0FFF);
        self.bg_next_tile_id = self.read_vram(addr, mapper);
    }

    fn fetch_attribute_byte(&mut self, mapper: &mut dyn crate::mapper::Mapper) {
        let addr = 0x23C0 | (self.v & 0x0C00) | ((self.v >> 4) & 0x38) | ((self.v >> 2) & 0x07);
        let attr = self.read_vram(addr, mapper);
        let shift = ((self.v >> 4) & 4) | (self.v & 2);
        self.bg_next_tile_attr = (attr >> shift) & 0x03;
    }

    fn fetch_pattern_low_byte(&mut self, mapper: &mut dyn crate::mapper::Mapper) {
        let fine_y = (self.v >> 12) & 0x07;
        let addr = self.ppu_ctrl.get_bg_pattern_table_addr()
            | ((self.bg_next_tile_id as u16) << 4)
            | fine_y;
        self.bg_next_tile_lo = self.read_vram(addr, mapper);
    }

    fn fetch_pattern_high_byte(&mut self, mapper: &mut dyn crate::mapper::Mapper) {
        let fine_y = (self.v >> 12) & 0x07;
        let addr = self.ppu_ctrl.get_bg_pattern_table_addr()
            | ((self.bg_next_tile_id as u16) << 4)
            | fine_y
            | 0x08;
        self.bg_next_tile_hi = self.read_vram(addr, mapper);
    }

    fn load_background_shifters(&mut self) {
        self.bg_shift_lo = (self.bg_shift_lo & 0xFF00) | self.bg_next_tile_lo as u16;
        self.bg_shift_hi = (self.bg_shift_hi & 0xFF00) | self.bg_next_tile_hi as u16;

        self.bg_attr_shift_lo = (self.bg_attr_shift_lo & 0xFF00)
            | if self.bg_next_tile_attr & 0x01 != 0 {
                0xFF
            } else {
                0x00
            };
        self.bg_attr_shift_hi = (self.bg_attr_shift_hi & 0xFF00)
            | if self.bg_next_tile_attr & 0x02 != 0 {
                0xFF
            } else {
                0x00
            };
    }

    fn update_shifters(&mut self) {
        if self.ppu_mask.show_background() {
            self.bg_shift_lo <<= 1;
            self.bg_shift_hi <<= 1;
            self.bg_attr_shift_lo <<= 1;
            self.bg_attr_shift_hi <<= 1;
        }

        if self.ppu_mask.show_sprites() && self.dot >= 1 && self.dot < 258 {
            for i in 0..self.sprite_count {
                if self.sprite_positions[i] > 0 {
                    self.sprite_positions[i] -= 1;
                } else {
                    self.sprite_patterns_lo[i] <<= 1;
                    self.sprite_patterns_hi[i] <<= 1;
                }
            }
        }
    }

    fn increment_scroll_x(&mut self) {
        if !self.ppu_mask.rendering_enabled() {
            return;
        }

        if (self.v & 0x001F) == 31 {
            self.v &= !0x001F;
            self.v ^= 0x0400;
        } else {
            self.v += 1;
        }
    }

    fn increment_scroll_y(&mut self) {
        if !self.ppu_mask.rendering_enabled() {
            return;
        }

        if (self.v & 0x7000) != 0x7000 {
            self.v += 0x1000;
        } else {
            self.v &= !0x7000;
            let mut y = (self.v & 0x03E0) >> 5;
            if y == 29 {
                y = 0;
                self.v ^= 0x0800;
            } else if y == 31 {
                y = 0;
            } else {
                y += 1;
            }
            self.v = (self.v & !0x03E0) | (y << 5);
        }
    }

    fn copy_horizontal_position(&mut self) {
        if !self.ppu_mask.rendering_enabled() {
            return;
        }
        self.v = (self.v & !0x041F) | (self.t & 0x041F);
    }

    fn copy_vertical_position(&mut self) {
        if !self.ppu_mask.rendering_enabled() {
            return;
        }
        self.v = (self.v & !0x7BE0) | (self.t & 0x7BE0);
    }

    fn evaluate_sprites(&mut self) {
        self.sprite_count = 0;
        self.secondary_oam = [0xFF; 32];

        let sprite_height = if self.ppu_ctrl.sprite_size() as u8 == 1 {
            16
        } else {
            8
        };

        for i in 0..64 {
            let sprite_y = self.oam[i * 4] as i16;
            let diff = self.scanline as i16 - sprite_y;

            if diff >= 0 && diff < sprite_height as i16 {
                if self.sprite_count < 8 {
                    for j in 0..4 {
                        self.secondary_oam[self.sprite_count * 4 + j] = self.oam[i * 4 + j];
                    }
                    self.sprite_indices[self.sprite_count] = i as u8;
                    self.sprite_count += 1;
                } else {
                    self.ppu_status.set_sprite_overflow(true);
                    break;
                }
            }
        }
    }

    fn load_sprite_data(&mut self, mapper: &mut dyn crate::mapper::Mapper) {
        let sprite_height = if self.ppu_ctrl.sprite_size() as u8 == 1 {
            16
        } else {
            8
        };

        for i in 0..self.sprite_count {
            let sprite_y = self.secondary_oam[i * 4] as u16;
            let tile_index = self.secondary_oam[i * 4 + 1];
            let attributes = self.secondary_oam[i * 4 + 2];
            let sprite_x = self.secondary_oam[i * 4 + 3];

            let mut row = (self.scanline as u16).wrapping_sub(sprite_y);

            if attributes & 0x80 != 0 {
                row = sprite_height - 1 - row;
            }

            let pattern_addr = if sprite_height == 16 {
                let bank = if tile_index & 0x01 != 0 {
                    0x1000
                } else {
                    0x0000
                };
                let tile = (tile_index & 0xFE) as u16;
                if row >= 8 {
                    bank | (tile << 4) | ((row - 8) & 0x07) | 0x10
                } else {
                    bank | (tile << 4) | (row & 0x07)
                }
            } else {
                let bank = self.ppu_ctrl.get_sprite_pattern_table_addr();
                bank | ((tile_index as u16) << 4) | row
            };

            let mut pattern_lo = self.read_vram(pattern_addr, mapper);
            let mut pattern_hi = self.read_vram(pattern_addr + 8, mapper);

            if attributes & 0x40 != 0 {
                pattern_lo = pattern_lo.reverse_bits();
                pattern_hi = pattern_hi.reverse_bits();
            }

            self.sprite_patterns_lo[i] = pattern_lo;
            self.sprite_patterns_hi[i] = pattern_hi;
            self.sprite_positions[i] = sprite_x;
            self.sprite_priorities[i] = (attributes >> 5) & 0x01;
        }
    }

    fn render_pixel(&mut self) {
        let x = self.dot - 1;
        let y = self.scanline;

        let mut bg_pixel = 0u8;
        let mut bg_palette = 0u8;

        if self.ppu_mask.show_background() && (self.ppu_mask.show_background_left() || x >= 8) {
            let bit_mux = 0x8000 >> self.x;
            let p0 = (self.bg_shift_lo & bit_mux) > 0;
            let p1 = (self.bg_shift_hi & bit_mux) > 0;
            bg_pixel = (p1 as u8) << 1 | p0 as u8;

            let bg_pal0 = (self.bg_attr_shift_lo & bit_mux) > 0;
            let bg_pal1 = (self.bg_attr_shift_hi & bit_mux) > 0;
            bg_palette = (bg_pal1 as u8) << 1 | bg_pal0 as u8;
        }

        let mut fg_pixel = 0u8;
        let mut fg_palette = 0u8;
        let mut fg_priority = 0u8;
        let mut sprite_zero_rendering = false;

        if self.ppu_mask.show_sprites() && (self.ppu_mask.show_sprites_left() || x >= 8) {
            for i in 0..self.sprite_count {
                if self.sprite_positions[i] == 0 {
                    let pixel_lo = (self.sprite_patterns_lo[i] & 0x80) > 0;
                    let pixel_hi = (self.sprite_patterns_hi[i] & 0x80) > 0;
                    fg_pixel = (pixel_hi as u8) << 1 | pixel_lo as u8;

                    if fg_pixel != 0 {
                        fg_palette = (self.secondary_oam[i * 4 + 2] & 0x03) + 4;
                        fg_priority = self.sprite_priorities[i];
                        sprite_zero_rendering = self.sprite_indices[i] == 0;
                        break;
                    }
                }
            }
        }

        let mut pixel = 0u8;
        let mut palette = 0u8;

        if bg_pixel == 0 && fg_pixel == 0 {
            pixel = 0;
            palette = 0;
        } else if bg_pixel == 0 && fg_pixel > 0 {
            pixel = fg_pixel;
            palette = fg_palette;
        } else if bg_pixel > 0 && fg_pixel == 0 {
            pixel = bg_pixel;
            palette = bg_palette;
        } else {
            if fg_priority == 0 {
                pixel = fg_pixel;
                palette = fg_palette;
            } else {
                pixel = bg_pixel;
                palette = bg_palette;
            }

            if sprite_zero_rendering
                && x < 255
                && self.ppu_mask.show_background()
                && self.ppu_mask.show_sprites()
            {
                if !(self.ppu_mask.show_background_left() || self.ppu_mask.show_sprites_left()) {
                    if x >= 8 {
                        self.ppu_status.set_sprite_0_hit(true);
                    }
                } else {
                    self.ppu_status.set_sprite_0_hit(true);
                }
            }
        }

        let color_addr = (palette as u16) * 4 + pixel as u16;
        let color = self.palette[color_addr as usize] & 0x3F;

        self.screen[y * 256 + x] = self.get_color_from_palette(color);
    }

    pub fn get_color_from_palette(&self, index: u8) -> u32 {
        const PALETTE: [u32; 64] = [
            0xFF666666, 0xFF002A88, 0xFF1412A7, 0xFF3B00A4, 0xFF5C007E, 0xFF6E0040, 0xFF6C0600,
            0xFF561D00, 0xFF333500, 0xFF0B4800, 0xFF005200, 0xFF004F08, 0xFF00404D, 0xFF000000,
            0xFF000000, 0xFF000000, 0xFFADADAD, 0xFF155FD9, 0xFF4240FF, 0xFF7527FE, 0xFFA01ACC,
            0xFFB71E7B, 0xFFB53120, 0xFF994E00, 0xFF6B6D00, 0xFF388700, 0xFF0C9300, 0xFF008F32,
            0xFF007C8D, 0xFF000000, 0xFF000000, 0xFF000000, 0xFFFFFEFF, 0xFF64B0FF, 0xFF9290FF,
            0xFFC676FF, 0xFFF36AFF, 0xFFFE6ECC, 0xFFFE8170, 0xFFEA9E22, 0xFFBCBE00, 0xFF88D800,
            0xFF5CE430, 0xFF45E082, 0xFF48CDDE, 0xFF4F4F4F, 0xFF000000, 0xFF000000, 0xFFFFFEFF,
            0xFFC0DFFF, 0xFFD3D2FF, 0xFFE8C8FF, 0xFFFBC2FF, 0xFFFEC4EA, 0xFFFECCC5, 0xFFF7D8A5,
            0xFFE4E594, 0xFFCFEF96, 0xFFBDF4AB, 0xFFB3F3CC, 0xFFB5EBF2, 0xFFB8B8B8, 0xFF000000,
            0xFF000000,
        ];
        PALETTE[index as usize]
    }

    pub fn read_vram(&mut self, addr: u16, mapper: &mut dyn crate::mapper::Mapper) -> u8 {
        let addr = addr & 0x3FFF;

        if addr < 0x2000 {
            return mapper.read_chr(addr);
        }

        if addr < 0x3F00 {
            let mirrored_addr = self.mirror_vram_addr(addr, mapper.mirroring());
            return self.vram[mirrored_addr];
        }

        if addr < 0x4000 {
            let palette_addr = ((addr - 0x3F00) % 32) as usize;
            let adjusted_addr = if palette_addr.is_multiple_of(4) && palette_addr >= 16 {
                palette_addr - 16
            } else {
                palette_addr
            };
            return self.palette[adjusted_addr];
        }

        0
    }

    pub fn write_vram(&mut self, addr: u16, value: u8, mapper: &mut dyn crate::mapper::Mapper) {
        let addr = addr & 0x3FFF;

        if addr < 0x2000 {
            mapper.write_chr(addr, value);
            return;
        }

        if addr < 0x3F00 {
            let mirrored_addr = self.mirror_vram_addr(addr, mapper.mirroring());
            self.vram[mirrored_addr] = value;
            return;
        }

        if addr < 0x4000 {
            let palette_addr = ((addr - 0x3F00) % 32) as usize;

            if palette_addr.is_multiple_of(4) {
                self.palette[palette_addr] = value;
                self.palette[palette_addr ^ 0x10] = value;
            } else {
                self.palette[palette_addr] = value;
            }
        }
    }

    fn mirror_vram_addr(&self, addr: u16, mirroring: crate::mapper::Mirroring) -> usize {
        let addr = ((addr - 0x2000) & 0x0FFF) as usize;
        let table = addr / 0x400;
        let offset = addr % 0x400;

        let mapped_table = match mirroring {
            crate::mapper::Mirroring::Horizontal => match table {
                0 | 1 => 0,
                2 | 3 => 1,
                _ => 0,
            },
            crate::mapper::Mirroring::Vertical => match table {
                0 | 2 => 0,
                1 | 3 => 1,
                _ => 0,
            },
            crate::mapper::Mirroring::SingleScreenLower => 0,
            crate::mapper::Mirroring::SingleScreenUpper => 1,
            crate::mapper::Mirroring::FourScreen => table,
        };

        mapped_table * 0x400 + offset
    }

    pub fn write_ctrl(&mut self, value: u8) {
        let prev_nmi = self.ppu_ctrl.vblank();
        self.ppu_ctrl.set(value);
        self.t = (self.t & !0x0C00) | ((value as u16 & 0x03) << 10);

        if !prev_nmi && self.ppu_ctrl.vblank() && self.ppu_status.vblank() {
            self.nmi_pending = true;
        } else if !self.ppu_ctrl.vblank() {
            self.nmi_pending = false;
        }
    }

    pub fn write_mask(&mut self, value: u8) {
        self.ppu_mask.set(value);
    }

    pub fn read_status(&mut self) -> u8 {
        let status = self.ppu_status.get();
        self.ppu_status.set_vblank(false);
        self.w = false;
        self.nmi_pending = false;
        status
    }

    pub fn write_oam_addr(&mut self, value: u8) {
        self.oam_addr = value;
    }

    pub fn read_oam_data(&self) -> u8 {
        self.oam[self.oam_addr as usize]
    }

    pub fn write_oam_data(&mut self, value: u8) {
        self.oam[self.oam_addr as usize] = value;
        self.oam_addr = self.oam_addr.wrapping_add(1);
    }

    pub fn write_scroll(&mut self, value: u8) {
        if self.w {
            self.t =
                (self.t & !0x73E0) | ((value as u16 & 0x07) << 12) | ((value as u16 & 0xF8) << 2);
            self.w = false;
        } else {
            self.t = (self.t & !0x001F) | ((value as u16) >> 3);
            self.x = value & 0x07;
            self.w = true;
        }
    }

    pub fn write_addr(&mut self, value: u8) {
        if self.w {
            self.t = (self.t & !0x00FF) | value as u16;
            self.v = self.t;
            self.w = false;
        } else {
            self.t = (self.t & !0xFF00) | ((value as u16 & 0x3F) << 8);
            self.w = true;
        }
    }

    pub fn read_data(&mut self, mapper: &mut dyn crate::mapper::Mapper) -> u8 {
        let addr = self.v;
        self.v = self
            .v
            .wrapping_add(self.ppu_ctrl.get_vram_addr_inc() as u16);

        if addr < 0x3F00 {
            let data = self.buffer;
            self.buffer = self.read_vram(addr, mapper);
            data
        } else {
            self.buffer = self.read_vram(addr & 0x2FFF, mapper);
            self.read_vram(addr, mapper)
        }
    }

    pub fn write_data(&mut self, value: u8, mapper: &mut dyn crate::mapper::Mapper) {
        let addr = self.v;
        self.write_vram(addr, value, mapper);
        self.v = self
            .v
            .wrapping_add(self.ppu_ctrl.get_vram_addr_inc() as u16);
    }

    pub fn write_oam_dma(&mut self, _page: u8, data: &[u8; 256]) {
        for (i, _) in data.iter().enumerate() {
            self.oam[(self.oam_addr.wrapping_add(i as u8)) as usize] = data[i];
        }
    }

    pub fn check_nmi(&mut self) -> bool {
        if self.nmi_pending {
            self.nmi_pending = false;
            true
        } else {
            false
        }
    }
}
