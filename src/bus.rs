use log::warn;

use crate::{cart::Cart, ppu::Ppu};

#[derive(Default, Clone)]
pub struct Controller {
    pub realtime: u8,
    latched: u8,
    index: u8,
    strobe: bool,
}

#[derive(Clone)]
pub struct Bus {
    mem: [u8; 0x10000],
    pub cart: Option<Cart>,
    pub controller1: Controller,
    pub controller2: Controller,
}

impl Default for Bus {
    fn default() -> Self {
        Self {
            mem: [0; 0x10000],
            cart: None,
            controller1: Default::default(),
            controller2: Default::default(), // TODO: process controller 2
        }
    }
}

impl Bus {
    pub fn new() -> Self {
        Default::default()
    }

    fn read_cartridge(&self, addr: u16) -> u8 {
        self.cart
            .as_ref()
            .map(|c| c.mapper.read_prg(addr))
            .unwrap_or(0)
    }

    pub fn read_byte(&self, addr: usize) -> u8 {
        match addr {
            0x6000..=0xFFFF => self.read_cartridge(addr as u16),
            _ => self.mem[addr],
        }
    }

    fn read_controller(&mut self) -> u8 {
        let data = if self.controller1.strobe {
            self.controller1.latched & 1
        } else if self.controller1.index < 8 {
            let val = (self.controller1.latched >> self.controller1.index) & 1;
            self.controller1.index += 1;
            val
        } else {
            1
        };
        data | 0x40
    }

    pub fn cpu_read_byte(&mut self, ppu: &mut Ppu, addr: usize) -> u8 {
        match addr {
            0x2000..=0x3FFF => {
                let reg = (addr - 0x2000) % 8;
                match reg {
                    2 => ppu.read_status(),
                    4 => ppu.read_oam_data(),
                    7 => self
                        .cart
                        .as_mut()
                        .map(|c| ppu.read_data(&mut *c.mapper))
                        .unwrap_or(0),
                    _ => self.mem[addr],
                }
            }
            0x4016 => self.read_controller(),
            0x4017 => 0x40,
            0x6000..=0xFFFF => self.read_cartridge(addr as u16),
            _ => self.mem[addr],
        }
    }

    pub fn read(&self, addr: u16, bytes: u16) -> Vec<u8> {
        (0..bytes)
            .map(|i| self.read_byte((addr + i) as usize))
            .collect()
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

    pub fn cpu_write_byte(&mut self, ppu: &mut Ppu, addr: usize, value: u8) {
        if addr < 0x6000 {
            self.mem[addr] = value;
        }

        match addr {
            0x2000..=0x3FFF => {
                let reg = (addr - 0x2000) % 8;
                match reg {
                    0 => ppu.write_ctrl(value),
                    1 => ppu.write_mask(value),
                    2 => warn!("Invalid write request to PPUSTATUS"),
                    3 => ppu.write_oam_addr(value),
                    4 => ppu.write_oam_data(value),
                    5 => ppu.write_scroll(value),
                    6 => ppu.write_addr(value),
                    7 => {
                        if let Some(cart) = &mut self.cart {
                            ppu.write_data(value, &mut *cart.mapper);
                        }
                    }
                    _ => unreachable!(),
                }
            }
            0x4014 => {
                let data: Vec<u8> = (0..256)
                    .map(|i| self.read_byte((value as usize) * 0x100 + i))
                    .collect();
                ppu.write_oam_dma(&data);
            }
            0x4016 => self.write_controller(value),
            0x6000..=0xFFFF => {
                if let Some(cart) = &mut self.cart {
                    cart.mapper.write_prg(addr as u16, value);
                }
            }
            _ => {}
        }
    }

    pub fn write_byte(&mut self, addr: usize, value: u8) {
        match addr {
            0x4016 => self.write_controller(value),
            0x6000..=0xFFFF => {
                if let Some(cart) = &mut self.cart {
                    cart.mapper.write_prg(addr as u16, value);
                }
            }
            _ => self.mem[addr] = value,
        }
    }

    pub fn write(&mut self, addr: usize, value: &[u8]) {
        for (i, &byte) in value.iter().enumerate() {
            self.write_byte(addr + i, byte);
        }
    }

    pub fn insert_cartridge(&mut self, cart: Cart) {
        self.cart = Some(cart);
    }
}
