use log::warn;
use savefile::prelude::*;

use crate::{apu::Apu, cart::Cart, ppu::Ppu};

#[derive(Default, Clone, Savefile)]
pub struct Controller {
    pub realtime: u8,
    latched: u8,
    index: u8,
    strobe: bool,
}

#[derive(Clone, Savefile)]
pub struct Bus {
    mem: [u8; 0x800],
    pub apu: Apu,
    pub ppu: Ppu,
    #[savefile_introspect_ignore]
    #[savefile_ignore]
    pub cart: Option<Cart>,
    pub controller1: Controller,
    pub controller2: Controller,
    pub open_bus: u8,
}

impl Default for Bus {
    fn default() -> Self {
        Self {
            mem: [0; 0x800],
            apu: Default::default(),
            ppu: Default::default(),
            cart: None,
            controller1: Default::default(),
            controller2: Default::default(), // TODO: process controller 2
            open_bus: 0,
        }
    }
}

impl Bus {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn tick_apu(&mut self) {
        self.apu.step();
        if let Some(addr) = self.apu.poll_dmc_dma() {
            let val = self.read_byte(addr);
            self.apu.fill_dmc_buffer(val);
        }
    }

    pub fn insert_cartridge(&mut self, cart: Cart) {
        self.cart = Some(cart);
    }

    fn read_mem(&self, addr: u16) -> u8 {
        self.mem[(addr & 0x7FF) as usize]
    }

    fn read_apu(&mut self) -> u8 {
        (self.apu.read_status() & 0b1101_1111) | (self.open_bus & 0b0010_0000)
    }

    fn read_ppu(&mut self, addr: u16) -> u8 {
        if self.ppu.frame > self.ppu.open_bus_decay_timer + 60 {
            self.ppu.open_bus = 0;
        }

        let reg = addr & 0x07;
        let result = match reg {
            2 => {
                let status = self.ppu.read_status();
                (status & 0xE0) | (self.ppu.open_bus & 0x1F)
            }
            4 => self.ppu.read_oam_data(),
            7 => self
                .cart
                .as_mut()
                .map(|c| self.ppu.read_data(&mut *c.mapper))
                .unwrap_or(0),
            _ => self.ppu.open_bus,
        };

        self.ppu.open_bus = result;
        result
    }

    fn read_cartridge(&self, addr: u16) -> u8 {
        self.cart
            .as_ref()
            .and_then(|c| c.mapper.read_prg(addr))
            .unwrap_or(self.open_bus)
    }

    fn read_controller1(&mut self) -> u8 {
        let data = if self.controller1.strobe {
            self.controller1.latched & 1
        } else if self.controller1.index < 8 {
            let val = (self.controller1.latched >> self.controller1.index) & 1;
            self.controller1.index += 1;
            val
        } else {
            1
        };
        (self.open_bus & 0xE0) | data
    }

    fn read_controller2(&mut self) -> u8 {
        let data = if self.controller2.strobe {
            self.controller2.latched & 1
        } else if self.controller2.index < 8 {
            let val = (self.controller2.latched >> self.controller2.index) & 1;
            self.controller2.index += 1;
            val
        } else {
            1
        };
        (self.open_bus & 0xE0) | data
    }

    pub fn read_byte(&mut self, addr: u16) -> u8 {
        let value = match addr {
            0x0000..=0x1FFF => self.read_mem(addr),
            0x2000..=0x3FFF => self.read_ppu(addr),
            0x4014 => self.open_bus,
            0x4015 => self.read_apu(),
            0x4016 => self.read_controller1(),
            0x4017 => self.read_controller2(),
            0x4000..=0x401F => self.open_bus,
            0x4020..=0xFFFF => self.read_cartridge(addr),
        };

        if addr != 0x4015 {
            self.open_bus = value;
        }
        value
    }

    pub fn read_only(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x1FFF => self.read_mem(addr),
            0x2000..=0x3FFF => 0,
            0x4020..=0xFFFF => self
                .cart
                .as_ref()
                .and_then(|c| c.mapper.read_prg(addr))
                .unwrap_or(self.open_bus),
            _ => 0,
        }
    }

    pub fn read_range(&mut self, addr: u16, bytes: u16) -> Vec<u8> {
        if bytes == 0 {
            return vec![];
        }
        let values: Vec<u8> = (0..bytes).map(|i| self.read_byte(addr + i)).collect();
        self.open_bus = values.last().copied().unwrap();
        values
    }

    pub fn read_only_range(&self, addr: u16, bytes: u16) -> Vec<u8> {
        (0..bytes).map(|i| self.read_only(addr + i)).collect()
    }

    fn write_mem(&mut self, addr: u16, value: u8) {
        self.mem[(addr & 0x7FF) as usize] = value;
    }

    fn write_apu(&mut self, addr: u16, value: u8) {
        self.apu.write_register(addr, value);
    }

    fn write_ppu(&mut self, addr: u16, value: u8) {
        self.ppu.open_bus = value;
        self.ppu.open_bus_decay_timer = self.ppu.frame;

        let reg = addr & 0x07;
        match reg {
            0 => self.ppu.write_ctrl(value),
            1 => self.ppu.write_mask(value),
            2 => warn!("Invalid write request to PPUSTATUS"),
            3 => self.ppu.write_oam_addr(value),
            4 => self.ppu.write_oam_data(value),
            5 => self.ppu.write_scroll(value),
            6 => self.ppu.write_addr(value),
            7 => {
                if let Some(cart) = &mut self.cart {
                    self.ppu.write_data(value, &mut *cart.mapper);
                }
            }
            _ => unreachable!(),
        }
    }

    fn write_controller(&mut self, value: u8) {
        let new_strobe = (value & 1) == 1;

        if new_strobe {
            self.controller1.latched = self.controller1.realtime;
            self.controller1.index = 0;
        } else if self.controller1.strobe {
            self.controller1.index = 0;
        }

        self.controller1.strobe = new_strobe;
    }

    fn write_dma(&mut self, value: u8) {
        let page = (value as u16) << 8;
        let mut data = [0u8; 256];
        for i in 0..256 {
            data[i as usize] = self.read_byte(page + i);
        }
        self.ppu.write_oam_dma(&data);
    }

    pub fn write_byte(&mut self, addr: u16, value: u8) {
        self.open_bus = value;

        match addr {
            0x0000..=0x1FFF => self.write_mem(addr, value),
            0x2000..=0x3FFF => self.write_ppu(addr, value),
            0x4014 => self.write_dma(value),
            0x4015 => self.write_apu(addr, value),
            0x4016 => self.write_controller(value),
            0x4017 => self.write_apu(addr, value),
            0x4000..=0x401F => self.write_apu(addr, value),
            0x4020..=0xFFFF => {
                if let Some(cart) = &mut self.cart {
                    cart.mapper.write_prg(addr, value);
                }
            }
        }
    }

    pub fn write_range(&mut self, addr: u16, value: &[u8]) {
        for (i, &byte) in value.iter().enumerate() {
            self.write_byte(addr + i as u16, byte);
        }
    }
}
