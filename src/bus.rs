use log::warn;

use crate::{cart::Cart, ppu::Ppu};

#[derive(Clone)]
pub struct Bus {
    mem: [u8; 0x10000],
    cart: Option<Cart>,
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

    pub fn read(&self, addr: u16, bytes: u16) -> Vec<u8> {
        let mut result = Vec::with_capacity(bytes as usize);
        for i in 0..bytes {
            result.push(self.read_byte((addr + i) as usize));
        }
        result
    }

    pub fn cpu_write_byte(&mut self, ppu: &mut Ppu, addr: usize, value: u8) {
        self.write_byte(addr, value);

        match addr {
            0x2000..=0x3FFF => {
                let register = (addr - 0x2000) % 8;
                match register {
                    0 => ppu.ppu_ctrl.set(value),
                    1 => ppu.ppu_mask.set(value),
                    2 => warn!("Invalid write request to PPUSTATUS"),
                    3 => ppu.oam_addr.set(value),
                    4 => ppu.oam_data.set(value),
                    5 => ppu.ppu_scroll.set(value, &mut ppu.write_toggle),
                    6 => ppu.ppu_addr.set(value, &mut ppu.write_toggle),
                    7 => ppu.ppu_data.set(value),
                    _ => unreachable!(),
                }
            }
            0x4014 => ppu.oam_dma.set(value),
            0x6000..=0xFFFF => {
                if let Some(cart) = &mut self.cart {
                    cart.mapper.write_prg(addr as u16, value);
                }
            }
            _ => self.mem[addr] = value,
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
