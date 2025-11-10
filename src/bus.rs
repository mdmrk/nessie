use log::warn;

use crate::{cart::Cart, ppu::Ppu};

#[derive(Clone)]
pub struct Bus {
    mem: [u8; 0x10000],
    pub cart: Option<Cart>,
}

impl Default for Bus {
    fn default() -> Self {
        Self::new()
    }
}

impl Bus {
    pub fn new() -> Self {
        Self {
            mem: [0; 0x10000],
            cart: None,
        }
    }

    pub fn read_byte(&self, addr: usize) -> u8 {
        match addr {
            0x6000..=0xFFFF => {
                if let Some(cart) = &self.cart {
                    cart.mapper.read_prg(addr as u16)
                } else {
                    0
                }
            }
            _ => self.mem[addr],
        }
    }

    pub fn cpu_read_byte(&mut self, ppu: &mut Ppu, addr: usize) -> u8 {
        match addr {
            0x2000..=0x3FFF => {
                let register = (addr - 0x2000) % 8;
                match register {
                    0 => self.mem[addr],
                    1 => self.mem[addr],
                    2 => ppu.read_status(),
                    3 => self.mem[addr],
                    4 => ppu.read_oam_data(),
                    5 => self.mem[addr],
                    6 => self.mem[addr],
                    7 => {
                        if let Some(cart) = &mut self.cart {
                            let mapper = &mut *cart.mapper as &mut dyn crate::mapper::Mapper;
                            ppu.read_data(mapper)
                        } else {
                            0
                        }
                    }
                    _ => unreachable!(),
                }
            }
            0x6000..=0xFFFF => {
                if let Some(cart) = &self.cart {
                    cart.mapper.read_prg(addr as u16)
                } else {
                    0
                }
            }
            _ => self.mem[addr],
        }
    }

    pub fn read(&self, addr: u16, bytes: u16) -> Vec<u8> {
        let mut result = Vec::with_capacity(bytes as usize);
        for i in 0..bytes {
            result.push(self.read_byte((addr + i) as usize));
        }
        result
    }

    pub fn cpu_write_byte(&mut self, ppu: &mut Ppu, addr: usize, value: u8) {
        if addr < 0x6000 {
            self.mem[addr] = value;
        }

        match addr {
            0x2000..=0x3FFF => {
                let register = (addr - 0x2000) % 8;
                match register {
                    0 => ppu.write_ctrl(value),
                    1 => ppu.write_mask(value),
                    2 => warn!("Invalid write request to PPUSTATUS"),
                    3 => ppu.write_oam_addr(value),
                    4 => ppu.write_oam_data(value),
                    5 => ppu.write_scroll(value),
                    6 => ppu.write_addr(value),
                    7 => {
                        if let Some(cart) = &mut self.cart {
                            let mapper = &mut *cart.mapper as &mut dyn crate::mapper::Mapper;
                            ppu.write_data(value, mapper);
                        }
                    }
                    _ => unreachable!(),
                }
            }
            0x4014 => {
                let page_start = (value as usize) * 0x100;
                let mut data = [0u8; 256];
                for (i, item) in data.iter_mut().enumerate() {
                    *item = self.read_byte(page_start + i);
                }
                ppu.write_oam_dma(value, &data);
            }
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
