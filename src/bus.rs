#[derive(Clone)]
pub struct Bus {
    mem: [u8; 0x10000],
}

impl Bus {
    pub fn new() -> Self {
        Self { mem: [0; 0x10000] }
    }

    pub fn read_byte(&self, addr: usize) -> u8 {
        self.mem[addr]
    }

    pub fn write_byte(&mut self, addr: usize, value: u8) {
        self.mem[addr] = value;
    }

    pub fn write(&mut self, addr: usize, value: &[u8]) {
        self.mem[addr..value.len() + addr].copy_from_slice(value);
    }
}
