use crate::cart::Cart;

#[derive(Clone)]
pub struct Bus {
    mem: [u8; 0xffff],
}

impl Bus {
    pub fn new() -> Self {
        Self { mem: [0; 0xffff] }
    }

    pub fn read_byte(&self, addr: usize, cart: &mut Cart) -> u8 {
        match addr {
            0x8000..=0xffff => cart.rom[addr - 0x8000],
            _ => 0,
        }
    }

    pub fn write_byte(&mut self, addr: usize, value: u8) {
        self.mem[addr] = value;
    }
}
