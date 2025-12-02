use egui::Color32;
use modular_bitfield::prelude::*;
use savefile::prelude::*;

#[bitfield(bytes = 1)]
#[derive(Debug, Clone, Default, Copy, Savefile)]
pub struct PpuCtrl {
    pub base_nametable_addr: B2,
    pub vram_addr_inc: B1,
    pub sprite_pattern_table: B1,
    pub bg_pattern_table: B1,
    pub sprite_size: B1,
    pub master_slave: B1,
    pub nmi_enable: bool,
}

#[bitfield(bytes = 1)]
#[derive(Debug, Clone, Default, Copy, Savefile)]
pub struct PpuMask {
    pub greyscale: bool,
    pub show_bg_left: bool,
    pub show_sprites_left: bool,
    pub show_bg: bool,
    pub show_sprites: bool,
    pub emphasize_red: bool,
    pub emphasize_green: bool,
    pub emphasize_blue: bool,
}

impl PpuMask {
    #[inline(always)]
    pub fn rendering_enabled(&self) -> bool {
        self.show_bg() || self.show_sprites()
    }
}

#[bitfield(bytes = 1)]
#[derive(Debug, Clone, Default, Copy, Savefile)]
pub struct PpuStatus {
    pub unused: B5,
    pub sprite_overflow: bool,
    pub sprite_0_hit: bool,
    pub vblank: bool,
}

#[derive(Debug, Clone, Copy, Default, Savefile)]
struct Sprite {
    y: u8,
    tile_index: u8,
    attributes: u8,
    x: u8,
    index: u8,
}

impl Sprite {
    #[inline(always)]
    fn palette(&self) -> u8 {
        self.attributes & 0x03
    }

    #[inline(always)]
    fn priority(&self) -> bool {
        self.attributes & 0x20 != 0
    }

    #[inline(always)]
    fn flip_h(&self) -> bool {
        self.attributes & 0x40 != 0
    }

    #[inline(always)]
    fn flip_v(&self) -> bool {
        self.attributes & 0x80 != 0
    }
}

#[derive(Debug, Savefile)]
pub struct Ppu {
    pub scanline: u16,
    pub dot: u16,
    pub frame: u64,
    pub frame_ready: bool,
    pub open_bus: u8,
    pub open_bus_decay_timer: u64,
    pub ctrl: PpuCtrl,
    pub mask: PpuMask,
    pub status: PpuStatus,
    pub oam: [u8; 256],
    pub oam_addr: u8,
    pub vram: [u8; 2048],
    pub palette: [u8; 32],
    pub v: u16,
    pub t: u16,
    pub x: u8,
    pub w: bool,
    bg_next_tile_id: u8,
    bg_next_tile_attrib: u8,
    bg_next_tile_lsb: u8,
    bg_next_tile_msb: u8,
    bg_shifter_pattern_lo: u16,
    bg_shifter_pattern_hi: u16,
    bg_shifter_attrib_lo: u16,
    bg_shifter_attrib_hi: u16,
    pub read_buffer: u8,
    pub nmi_pending: bool,
    pub nmi_just_enabled: bool,
    pub suppress_nmi: bool,
    pub suppress_vbl: bool,
    pub nmi_delay: bool,
    #[savefile_introspect_ignore]
    #[savefile_ignore]
    pub screen: Vec<Color32>,
    secondary_oam: [u8; 32],
    sprites: [Sprite; 8],
    sprite_height: u16,
}

impl Clone for Ppu {
    fn clone(&self) -> Self {
        Self {
            scanline: self.scanline,
            dot: self.dot,
            frame: self.frame,
            frame_ready: self.frame_ready,
            open_bus: 0,
            open_bus_decay_timer: 0,
            ctrl: self.ctrl,
            mask: self.mask,
            status: self.status,
            oam: self.oam,
            oam_addr: self.oam_addr,
            vram: self.vram,
            palette: self.palette,
            v: self.v,
            t: self.t,
            x: self.x,
            w: self.w,
            bg_next_tile_id: self.bg_next_tile_id,
            bg_next_tile_attrib: self.bg_next_tile_attrib,
            bg_next_tile_lsb: self.bg_next_tile_lsb,
            bg_next_tile_msb: self.bg_next_tile_msb,
            bg_shifter_pattern_lo: self.bg_shifter_pattern_lo,
            bg_shifter_pattern_hi: self.bg_shifter_pattern_hi,
            bg_shifter_attrib_lo: self.bg_shifter_attrib_lo,
            bg_shifter_attrib_hi: self.bg_shifter_attrib_hi,
            read_buffer: self.read_buffer,
            nmi_pending: self.nmi_pending,
            nmi_just_enabled: self.nmi_just_enabled,
            suppress_nmi: self.suppress_nmi,
            suppress_vbl: self.suppress_vbl,
            nmi_delay: self.nmi_delay,
            screen: Vec::new(),
            secondary_oam: self.secondary_oam,
            sprites: self.sprites,
            sprite_height: self.sprite_height,
        }
    }
}

impl Default for Ppu {
    fn default() -> Self {
        Self {
            scanline: 0,
            dot: 0,
            frame: 0,
            frame_ready: false,
            open_bus: 0,
            open_bus_decay_timer: 0,
            ctrl: Default::default(),
            mask: Default::default(),
            status: Default::default(),
            oam: [0; 256],
            oam_addr: 0,
            vram: [0; 2048],
            palette: [0; 32],
            v: 0,
            t: 0,
            x: 0,
            w: false,
            bg_next_tile_id: 0,
            bg_next_tile_attrib: 0,
            bg_next_tile_lsb: 0,
            bg_next_tile_msb: 0,
            bg_shifter_pattern_lo: 0,
            bg_shifter_pattern_hi: 0,
            bg_shifter_attrib_lo: 0,
            bg_shifter_attrib_hi: 0,
            read_buffer: 0,
            nmi_pending: false,
            nmi_just_enabled: false,
            suppress_nmi: false,
            suppress_vbl: false,
            nmi_delay: false,
            screen: vec![Color32::BLACK; 256 * 240],
            secondary_oam: [0xFF; 32],
            sprites: [Sprite::default(); 8],
            sprite_height: 8,
        }
    }
}

impl Ppu {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) {
        self.ctrl = PpuCtrl::new();
        self.mask = PpuMask::new();
        self.status = PpuStatus::new();
        self.w = false;
        self.read_buffer = 0;
        self.scanline = 0;
        self.dot = 0;
        self.oam_addr = 0;
        self.suppress_nmi = false;
        self.suppress_vbl = false;
        self.nmi_delay = false;
        self.sprite_height = 8;
        self.bg_next_tile_id = 0;
        self.bg_next_tile_attrib = 0;
        self.bg_next_tile_lsb = 0;
        self.bg_next_tile_msb = 0;
        self.bg_shifter_pattern_lo = 0;
        self.bg_shifter_pattern_hi = 0;
        self.bg_shifter_attrib_lo = 0;
        self.bg_shifter_attrib_hi = 0;
    }

    pub fn step(&mut self, mapper: &mut dyn crate::mapper::Mapper, cpu_cycles: u8) {
        for _ in 0..(cpu_cycles * 3) {
            self.tick(mapper);
        }
    }

    pub fn tick(&mut self, mapper: &mut dyn crate::mapper::Mapper) {
        if self.scanline == 241 && self.dot == 1 {
            if !self.suppress_vbl {
                self.status.set_vblank(true);
            }
            if !self.suppress_nmi && self.ctrl.nmi_enable() {
                self.nmi_pending = true;
            }
            self.suppress_vbl = false;
            self.suppress_nmi = false;
        }

        if self.scanline < 240 || self.scanline == 261 {
            if (self.dot >= 1 && self.dot <= 256) || (self.dot >= 321 && self.dot <= 336) {
                self.update_shifters();
                self.process_bg_pipeline(mapper);
            }

            if self.scanline < 240 {
                if self.dot == 1 {
                    self.sprite_height = if self.ctrl.sprite_size() != 0 { 16 } else { 8 };
                    self.clear_secondary_oam();
                    self.fetch_sprites();
                }

                if self.dot >= 1 && self.dot <= 256 {
                    self.render_pixel(mapper);
                }
            }

            if self.mask.rendering_enabled() {
                if self.dot == 256 {
                    self.increment_y();
                } else if self.dot == 257 {
                    self.load_sprites(mapper);
                    self.copy_horizontal();
                } else if self.scanline == 261 && self.dot >= 280 && self.dot <= 304 {
                    self.copy_vertical();
                }
            }
        }

        if self.scanline == 261 && self.dot == 1 {
            self.status.set_vblank(false);
            self.status.set_sprite_0_hit(false);
            self.status.set_sprite_overflow(false);
            self.nmi_pending = false;
            self.suppress_nmi = false;
            self.suppress_vbl = false;
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

    fn process_bg_pipeline(&mut self, mapper: &mut dyn crate::mapper::Mapper) {
        if !self.mask.rendering_enabled() {
            return;
        }

        match (self.dot - 1) & 7 {
            0 => {
                self.load_background_shifters();
                let tile_addr = 0x2000 | (self.v & 0x0FFF);
                self.bg_next_tile_id = self.read_vram(tile_addr, mapper);
            }
            2 => {
                let attr_addr =
                    0x23C0 | (self.v & 0x0C00) | ((self.v >> 4) & 0x38) | ((self.v >> 2) & 0x07);
                let attr_byte = self.read_vram(attr_addr, mapper);
                let shift = ((self.v >> 4) & 4) | (self.v & 2);
                self.bg_next_tile_attrib = (attr_byte >> shift) & 0x03;
            }
            4 => {
                let pattern_table = (self.ctrl.bg_pattern_table() as u16) << 12;
                let fine_y = (self.v >> 12) & 0x07;
                let pattern_addr = pattern_table | ((self.bg_next_tile_id as u16) << 4) | fine_y;
                self.bg_next_tile_lsb = self.read_vram(pattern_addr, mapper);
            }
            6 => {
                let pattern_table = (self.ctrl.bg_pattern_table() as u16) << 12;
                let fine_y = (self.v >> 12) & 0x07;
                let pattern_addr = pattern_table | ((self.bg_next_tile_id as u16) << 4) | fine_y;
                self.bg_next_tile_msb = self.read_vram(pattern_addr + 8, mapper);
            }
            7 => self.increment_coarse_x(),
            _ => {}
        }
    }

    #[inline]
    fn update_shifters(&mut self) {
        if self.mask.rendering_enabled() {
            self.bg_shifter_pattern_lo <<= 1;
            self.bg_shifter_pattern_hi <<= 1;
            self.bg_shifter_attrib_lo <<= 1;
            self.bg_shifter_attrib_hi <<= 1;
        }
    }

    fn load_background_shifters(&mut self) {
        self.bg_shifter_pattern_lo =
            (self.bg_shifter_pattern_lo & 0xFF00) | self.bg_next_tile_lsb as u16;
        self.bg_shifter_pattern_hi =
            (self.bg_shifter_pattern_hi & 0xFF00) | self.bg_next_tile_msb as u16;

        let attrib_lo = if (self.bg_next_tile_attrib & 0x01) != 0 {
            0xFF
        } else {
            0x00
        };
        let attrib_hi = if (self.bg_next_tile_attrib & 0x02) != 0 {
            0xFF
        } else {
            0x00
        };

        self.bg_shifter_attrib_lo = (self.bg_shifter_attrib_lo & 0xFF00) | attrib_lo;
        self.bg_shifter_attrib_hi = (self.bg_shifter_attrib_hi & 0xFF00) | attrib_hi;
    }

    fn get_bg_pixel(&self) -> (u8, u8) {
        if !self.mask.show_bg() {
            return (0, 0);
        }

        let bit_mux = 0x8000 >> self.x;
        let p0 = (self.bg_shifter_pattern_lo & bit_mux) != 0;
        let p1 = (self.bg_shifter_pattern_hi & bit_mux) != 0;
        let pixel = ((p1 as u8) << 1) | (p0 as u8);

        if pixel == 0 {
            return (0, 0);
        }

        let a0 = (self.bg_shifter_attrib_lo & bit_mux) != 0;
        let a1 = (self.bg_shifter_attrib_hi & bit_mux) != 0;
        let palette = ((a1 as u8) << 1) | (a0 as u8);

        (pixel, palette)
    }

    fn get_sprite_pixel(&self, mapper: &mut dyn crate::mapper::Mapper) -> (u8, u8, u8, bool) {
        if !self.mask.show_sprites() {
            return (0, 0, 0, false);
        }

        let x = self.dot - 1;
        if x < 8 && !self.mask.show_sprites_left() {
            return (0, 0, 0, false);
        }

        for sprite in self.sprites.iter() {
            if sprite.y == 0xFF {
                break;
            }

            let sprite_x = sprite.x as u16;
            if x < sprite_x || x >= sprite_x + 8 {
                continue;
            }

            let mut fine_x = (x - sprite_x) as u8;
            let mut fine_y = (self.scanline.wrapping_sub(sprite.y as u16).wrapping_sub(1)) as u8;

            if sprite.flip_h() {
                fine_x = 7 - fine_x;
            }

            let mut tile_index = sprite.tile_index as u16;
            let pattern_table;

            if self.sprite_height == 16 {
                pattern_table = (tile_index & 0x01) << 12;
                tile_index &= 0xFE;
                if fine_y >= 8 {
                    fine_y -= 8;
                    if !sprite.flip_v() {
                        tile_index += 1;
                    }
                } else if sprite.flip_v() {
                    tile_index += 1;
                }
            } else {
                pattern_table = (self.ctrl.sprite_pattern_table() as u16) << 12;
                if sprite.flip_v() {
                    fine_y = 7 - fine_y;
                }
            }

            let pattern_addr = pattern_table | (tile_index << 4) | (fine_y as u16);
            let pattern_lo = self.read_vram(pattern_addr, mapper);
            let pattern_hi = self.read_vram(pattern_addr + 8, mapper);

            let bit_offset = 7 - fine_x;
            let pixel = (((pattern_hi >> bit_offset) & 1) << 1) | ((pattern_lo >> bit_offset) & 1);

            if pixel != 0 {
                return (
                    pixel,
                    (sprite.palette() << 2) | pixel,
                    sprite.index,
                    sprite.index == 0,
                );
            }
        }
        (0, 0, 0, false)
    }

    fn render_pixel(&mut self, mapper: &mut dyn crate::mapper::Mapper) {
        let x = self.dot.wrapping_sub(1) as usize;
        let y = self.scanline as usize;

        let (bg_pixel, bg_palette) = if x < 8 && !self.mask.show_bg_left() {
            (0, 0)
        } else {
            self.get_bg_pixel()
        };

        let (sp_pixel, sp_palette_addr_offset, sp_index, is_sprite_0) =
            self.get_sprite_pixel(mapper);

        let bg_palette_addr_offset = if bg_pixel > 0 {
            (bg_palette << 2) | bg_pixel
        } else {
            0
        };

        let final_color_index = match (bg_pixel, sp_pixel) {
            (0, 0) => 0,
            (0, _) => 0x10 | sp_palette_addr_offset,
            (_, 0) => bg_palette_addr_offset,
            (_, _) => {
                if is_sprite_0 && x < 255 && self.scanline < 240 {
                    self.status.set_sprite_0_hit(true);
                }
                let sprite = self.sprites.iter().find(|s| s.index == sp_index).unwrap();
                if sprite.priority() {
                    bg_palette_addr_offset
                } else {
                    0x10 | sp_palette_addr_offset
                }
            }
        };

        let final_palette_index = if (final_color_index & 0x03) == 0 {
            0
        } else {
            final_color_index & 0x1F
        };

        let color_index = self.palette[final_palette_index as usize] & 0x3F;
        self.screen[y * 256 + x] = Ppu::get_color_from_palette(color_index);
    }

    fn clear_secondary_oam(&mut self) {
        self.secondary_oam.fill(0xFF);
        self.sprites.iter_mut().for_each(|s| s.y = 0xFF);
    }

    fn fetch_sprites(&mut self) {
        let mut n = 0;
        let sprite_height = self.sprite_height;
        let oam_raw = &self.oam;

        for i in 0..64 {
            let idx = i * 4;
            let y = oam_raw[idx] as u16;

            if self.scanline > y && self.scanline <= y.wrapping_add(sprite_height) {
                if n < 8 {
                    let dst = n * 4;
                    self.secondary_oam[dst..dst + 4].copy_from_slice(&oam_raw[idx..idx + 4]);
                    self.sprites[n] = Sprite {
                        y: oam_raw[idx],
                        tile_index: oam_raw[idx + 1],
                        attributes: oam_raw[idx + 2],
                        x: oam_raw[idx + 3],
                        index: i as u8,
                    };
                    n += 1;
                } else {
                    self.status.set_sprite_overflow(true);
                    break;
                }
            }
        }
        for i in n..8 {
            self.sprites[i].y = 0xFF;
        }
    }

    fn load_sprites(&mut self, _mapper: &mut dyn crate::mapper::Mapper) {}

    #[inline]
    fn increment_coarse_x(&mut self) {
        if (self.v & 0x001F) == 31 {
            self.v &= !0x001F;
            self.v ^= 0x0400;
        } else {
            self.v += 1;
        }
    }

    #[inline]
    fn increment_y(&mut self) {
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

    #[inline]
    fn copy_horizontal(&mut self) {
        self.v = (self.v & !0x041F) | (self.t & 0x041F);
    }

    #[inline]
    fn copy_vertical(&mut self) {
        self.v = (self.v & !0x7BE0) | (self.t & 0x7BE0);
    }

    pub fn read_vram(&self, addr: u16, mapper: &mut dyn crate::mapper::Mapper) -> u8 {
        let addr = addr & 0x3FFF;
        if addr < 0x2000 {
            mapper.read_chr(addr)
        } else if addr < 0x3F00 {
            self.vram[self.mirror_vram_addr(addr, mapper.mirroring())]
        } else if addr < 0x4000 {
            self.palette[(addr & 0x1F) as usize] & 0x3F
        } else {
            0
        }
    }

    pub fn write_vram(&mut self, addr: u16, value: u8, mapper: &mut dyn crate::mapper::Mapper) {
        let addr = addr & 0x3FFF;
        if addr < 0x2000 {
            mapper.write_chr(addr, value);
        } else if addr < 0x3F00 {
            let m_addr = self.mirror_vram_addr(addr, mapper.mirroring());
            self.vram[m_addr] = value;
        } else if addr < 0x4000 {
            let p_addr = (addr & 0x1F) as usize;
            let value = value & 0x3F;
            self.palette[p_addr] = value;
            if (p_addr & 0x03) == 0 {
                self.palette[p_addr ^ 0x10] = value;
            }
        }
    }

    fn mirror_vram_addr(&self, addr: u16, mirroring: crate::mapper::Mirroring) -> usize {
        let addr = (addr & 0x0FFF) as usize;
        let table = addr >> 10;
        let offset = addr & 0x3FF;

        let mapped_table = match mirroring {
            crate::mapper::Mirroring::Horizontal => table >> 1,
            crate::mapper::Mirroring::Vertical => table & 1,
            crate::mapper::Mirroring::SingleScreenLower => 0,
            crate::mapper::Mirroring::SingleScreenUpper => 1,
            crate::mapper::Mirroring::FourScreen => table,
        };
        (mapped_table << 10) | offset
    }

    pub fn write_ctrl(&mut self, value: u8) {
        let prev_nmi = self.ctrl.nmi_enable();
        self.ctrl = PpuCtrl::from_bytes([value]);
        self.t = (self.t & !0x0C00) | ((value as u16 & 0x03) << 10);

        if !prev_nmi && self.ctrl.nmi_enable() && self.status.vblank() {
            self.nmi_pending = true;
            self.nmi_delay = true;
        } else if !self.ctrl.nmi_enable() {
            self.nmi_pending = false;
        }
    }

    pub fn write_mask(&mut self, value: u8) {
        self.mask = PpuMask::from_bytes([value]);
    }

    pub fn read_status(&mut self) -> u8 {
        let mut status_byte = self.status.bytes[0];
        if self.scanline == 241 && self.dot == 1 {
            self.suppress_vbl = true;
            status_byte &= !0x80;
        }
        self.status.set_vblank(false);
        self.nmi_pending = false;
        self.w = false;
        if self.scanline == 241 && (self.dot == 1 || self.dot == 2) {
            self.suppress_nmi = true;
        }

        status_byte
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
        if !self.w {
            self.t = (self.t & !0x001F) | ((value as u16) >> 3);
            self.x = value & 0x07;
            self.w = true;
        } else {
            self.t =
                (self.t & !0x73E0) | ((value as u16 & 0x07) << 12) | ((value as u16 & 0xF8) << 2);
            self.w = false;
        }
    }

    pub fn write_addr(&mut self, value: u8) {
        if !self.w {
            self.t = (self.t & !0xFF00) | ((value as u16 & 0x3F) << 8);
            self.w = true;
        } else {
            self.t = (self.t & !0x00FF) | (value as u16);
            self.v = self.t;
            self.w = false;
        }
    }

    pub fn read_data(&mut self, mapper: &mut dyn crate::mapper::Mapper) -> u8 {
        let addr = self.v;
        let inc = if self.ctrl.vram_addr_inc() != 0 {
            32
        } else {
            1
        };
        self.v = self.v.wrapping_add(inc);

        if addr < 0x3F00 {
            let data = self.read_buffer;
            self.read_buffer = self.read_vram(addr, mapper);
            data
        } else {
            self.read_buffer = self.read_vram(addr & 0x2FFF, mapper);
            (self.read_vram(addr, mapper) & 0x3F) | (self.open_bus & 0xC0)
        }
    }

    pub fn write_data(&mut self, value: u8, mapper: &mut dyn crate::mapper::Mapper) {
        let addr = self.v;
        self.write_vram(addr, value, mapper);
        let inc = if self.ctrl.vram_addr_inc() != 0 {
            32
        } else {
            1
        };
        self.v = self.v.wrapping_add(inc);
    }

    pub fn write_oam_dma(&mut self, data: &[u8]) {
        for (i, &byte) in data.iter().enumerate() {
            self.oam[self.oam_addr.wrapping_add(i as u8) as usize] = byte;
        }
    }

    pub fn check_nmi(&mut self) -> bool {
        if self.nmi_delay {
            self.nmi_delay = false;
            return false;
        }
        if self.scanline == 241 && (self.dot == 1 || self.dot == 2) {
            return false;
        }
        if self.nmi_pending {
            self.nmi_pending = false;
            true
        } else {
            false
        }
    }

    pub fn get_color_from_palette(index: u8) -> Color32 {
        const PALETTE_COLORS: [u32; 64] = [
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

        let c = PALETTE_COLORS[(index & 0x3F) as usize];
        let [a, r, g, b] = c.to_be_bytes();
        Color32::from_rgba_unmultiplied(r, g, b, a)
    }
}
