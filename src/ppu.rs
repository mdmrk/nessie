use modular_bitfield::prelude::*;

// PPU Control Register ($2000)
#[bitfield(bytes = 1)]
#[derive(Debug, Clone, Default, Copy)]
pub struct PpuCtrl {
    pub base_nametable_addr: B2, // 0-1: Base nametable address (0 = $2000; 1 = $2400; 2 = $2800; 3 = $2C00)
    pub vram_addr_inc: B1,       // 2: VRAM address increment (0: add 1; 1: add 32)
    pub sprite_pattern_table: B1, // 3: Sprite pattern table address (0: $0000; 1: $1000)
    pub bg_pattern_table: B1,    // 4: Background pattern table address (0: $0000; 1: $1000)
    pub sprite_size: B1,         // 5: Sprite size (0: 8x8; 1: 8x16)
    pub master_slave: B1,        // 6: PPU master/slave select
    pub nmi_enable: bool,        // 7: Generate NMI at start of vblank
}

// PPU Mask Register ($2001)
#[bitfield(bytes = 1)]
#[derive(Debug, Clone, Default, Copy)]
pub struct PpuMask {
    pub greyscale: bool,         // 0: Greyscale
    pub show_bg_left: bool,      // 1: Show background in leftmost 8 pixels
    pub show_sprites_left: bool, // 2: Show sprites in leftmost 8 pixels
    pub show_bg: bool,           // 3: Show background
    pub show_sprites: bool,      // 4: Show sprites
    pub emphasize_red: bool,     // 5: Emphasize red
    pub emphasize_green: bool,   // 6: Emphasize green
    pub emphasize_blue: bool,    // 7: Emphasize blue
}

impl PpuMask {
    pub fn rendering_enabled(&self) -> bool {
        self.show_bg() || self.show_sprites()
    }
}

// PPU Status Register ($2002)
#[bitfield(bytes = 1)]
#[derive(Debug, Clone, Default, Copy)]
pub struct PpuStatus {
    pub unused: B5,            // 0-4: Not used
    pub sprite_overflow: bool, // 5: Sprite overflow
    pub sprite_0_hit: bool,    // 6: Sprite 0 hit
    pub vblank: bool,          // 7: Vertical blank has started
}

#[derive(Clone, Debug)]
pub struct Ppu {
    pub scanline: u16,
    pub dot: u16,
    pub frame: u64,
    pub frame_ready: bool,

    pub ctrl: PpuCtrl,
    pub mask: PpuMask,
    pub status: PpuStatus,

    pub oam: [u8; 256],
    pub oam_addr: u8,

    pub vram: [u8; 2048],
    pub palette: [u8; 32],

    pub v: u16,  // Current VRAM address (15 bits)
    pub t: u16,  // Temporary VRAM address (15 bits)
    pub x: u8,   // Fine X scroll (3 bits)
    pub w: bool, // Write toggle (1 bit)

    pub read_buffer: u8,

    pub nmi_pending: bool,
    pub nmi_just_enabled: bool,
    pub suppress_nmi: bool,
    pub suppress_vbl: bool,
    pub nmi_delay: bool,

    pub screen: Vec<u32>,
}

impl Default for Ppu {
    fn default() -> Self {
        Self {
            scanline: 0,
            dot: 0,
            frame: 0,
            frame_ready: false,
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
            read_buffer: 0,
            nmi_pending: false,
            nmi_just_enabled: false,
            suppress_nmi: false,
            suppress_vbl: false,
            nmi_delay: false,
            screen: vec![0; 256 * 240],
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
        if self.scanline < 240 {
            self.visible_scanline(mapper);
        }
        if self.scanline == 261 {
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

    pub fn step(&mut self, mapper: &mut dyn crate::mapper::Mapper, cpu_cycles: u8) {
        for _ in 0..(cpu_cycles * 3) {
            self.tick(mapper);
        }
    }

    fn visible_scanline(&mut self, mapper: &mut dyn crate::mapper::Mapper) {
        if self.dot >= 1 && self.dot <= 256 {
            self.render_pixel(mapper);

            if self.mask.rendering_enabled() && self.dot.is_multiple_of(8) {
                self.increment_coarse_x();
            }
        }

        if self.dot == 256 && self.mask.rendering_enabled() {
            self.increment_y();
        }

        if self.dot == 257 && self.mask.rendering_enabled() {
            self.copy_horizontal();
        }
    }

    fn prerender_scanline(&mut self, _mapper: &mut dyn crate::mapper::Mapper) {
        if self.dot == 0 {
            self.status.set_vblank(false);
            self.status.set_sprite_0_hit(false);
            self.status.set_sprite_overflow(false);
            self.nmi_pending = false;
            self.suppress_nmi = false;
            self.suppress_vbl = false;
        }

        if self.dot >= 280 && self.dot <= 304 && self.mask.rendering_enabled() {
            self.copy_vertical();
        }

        if self.dot == 257 && self.mask.rendering_enabled() {
            self.copy_horizontal();
        }
    }

    fn render_pixel(&mut self, mapper: &mut dyn crate::mapper::Mapper) {
        let x = (self.dot - 1) as usize;
        let y = self.scanline as usize;

        if !self.mask.show_bg() {
            self.screen[y * 256 + x] = self.get_color_from_palette(0);
            return;
        }

        if x < 8 && !self.mask.show_bg_left() {
            self.screen[y * 256 + x] = self.get_color_from_palette(0);
            return;
        }

        let fine_x = ((self.x as u16 + x as u16) % 8) as u8;

        let tile_addr = 0x2000 | (self.v & 0x0FFF);
        let tile_id = self.read_vram(tile_addr, mapper);

        let attr_addr =
            0x23C0 | (self.v & 0x0C00) | ((self.v >> 4) & 0x38) | ((self.v >> 2) & 0x07);
        let attr = self.read_vram(attr_addr, mapper);

        let shift = ((self.v >> 4) & 4) | (self.v & 2);
        let palette_high = (attr >> shift) & 0x03;

        let fine_y = (self.v >> 12) & 0x07;
        let pattern_table = if self.ctrl.bg_pattern_table() != 0 {
            0x1000
        } else {
            0x0000
        };
        let pattern_addr = pattern_table | ((tile_id as u16) << 4) | fine_y;

        let pattern_lo = self.read_vram(pattern_addr, mapper);
        let pattern_hi = self.read_vram(pattern_addr + 8, mapper);

        let bit_offset = 7 - fine_x;
        let pixel_lo = (pattern_lo >> bit_offset) & 1;
        let pixel_hi = (pattern_hi >> bit_offset) & 1;
        let pixel = (pixel_hi << 1) | pixel_lo;

        let palette_addr = if pixel == 0 {
            0
        } else {
            (palette_high << 2) | pixel
        };

        let color_index = self.palette[palette_addr as usize] & 0x3F;
        self.screen[y * 256 + x] = self.get_color_from_palette(color_index);
    }

    fn increment_coarse_x(&mut self) {
        if (self.v & 0x001F) == 31 {
            self.v &= !0x001F;
            self.v ^= 0x0400;
        } else {
            self.v += 1;
        }
    }

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

    fn copy_horizontal(&mut self) {
        self.v = (self.v & !0x041F) | (self.t & 0x041F);
    }

    fn copy_vertical(&mut self) {
        self.v = (self.v & !0x7BE0) | (self.t & 0x7BE0);
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
            let adjusted_addr = match palette_addr {
                0x10 => 0x00,
                0x14 => 0x04,
                0x18 => 0x08,
                0x1C => 0x0C,
                _ => palette_addr,
            };
            return self.palette[adjusted_addr] & 0x3F;
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

        if self.scanline == 241 {
            if self.dot == 0 {
                self.suppress_vbl = true;
                self.suppress_nmi = true;
            } else if self.dot == 1 {
                self.suppress_nmi = true;
                self.suppress_vbl = true;
                status_byte |= 0b1000_0000;
            } else if self.dot == 2 {
                self.nmi_pending = false;
            }
        }
        self.status.set_vblank(false);
        self.w = false;

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
        let increment = if self.ctrl.vram_addr_inc() != 0 {
            32
        } else {
            1
        };
        self.v = self.v.wrapping_add(increment);

        if addr < 0x3F00 {
            let data = self.read_buffer;
            self.read_buffer = self.read_vram(addr, mapper);
            data
        } else {
            self.read_buffer = self.read_vram(addr & 0x2FFF, mapper);
            self.read_vram(addr, mapper)
        }
    }

    pub fn write_data(&mut self, value: u8, mapper: &mut dyn crate::mapper::Mapper) {
        let addr = self.v;
        self.write_vram(addr, value, mapper);
        let increment = if self.ctrl.vram_addr_inc() != 0 {
            32
        } else {
            1
        };
        self.v = self.v.wrapping_add(increment);
    }

    pub fn write_oam_dma(&mut self, data: &[u8; 256]) {
        for (i, &byte) in data.iter().enumerate() {
            self.oam[(self.oam_addr.wrapping_add(i as u8)) as usize] = byte;
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
        PALETTE[index as usize & 0x3F]
    }
}
