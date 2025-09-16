#[derive(Clone)]
pub struct Bus {
    mem: [u8; 0xffff],
}

impl Bus {
    pub fn new() -> Self {
        Self { mem: [0; 0xffff] }
    }

    pub fn read_byte(&self, addr: usize) -> u8 {
        self.mem[addr]
    }

    pub fn write_byte(&mut self, addr: usize, value: u8) {
        self.mem[addr] = value;
    }
}
