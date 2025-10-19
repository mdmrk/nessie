use log::warn;

use crate::ppu::Ppu;

#[derive(Clone)]
pub struct Bus {
    mem: [u8; 0x10000],
}

impl Default for Bus {
    fn default() -> Self {
        Self::new()
    }
}

impl Bus {
    pub fn new() -> Self {
        Self { mem: [0; 0x10000] }
    }

    pub fn read_byte(&self, addr: usize) -> u8 {
        self.mem[addr]
    }

    pub fn read(&self, addr: u16, bytes: u16) -> &[u8] {
        &self.mem[addr as usize..addr as usize + bytes as usize]
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
            _ => {}
        }
    }

    pub fn write_byte(&mut self, addr: usize, value: u8) {
        self.mem[addr] = value;
    }

    pub fn write(&mut self, addr: usize, value: &[u8]) {
        self.mem[addr..value.len() + addr].copy_from_slice(value);
    }
}
