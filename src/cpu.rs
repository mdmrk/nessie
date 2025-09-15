use core::fmt;

bitflags::bitflags! {
    #[derive(Debug, Clone)]
    pub struct Flags: u8 {
        const N = 1 << 7;
        const V = 1 << 6;
        const _1 = 1 << 5;
        const B = 1 << 4;
        const D = 1 << 3;
        const I = 1 << 2;
        const Z = 1 << 1;
        const C = 1 << 0;
    }
}

impl fmt::Display for Flags {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut flags = Vec::new();

        if self.contains(Flags::N) {
            flags.push("N");
        }
        if self.contains(Flags::V) {
            flags.push("V");
        }
        if self.contains(Flags::_1) {
            flags.push("1");
        }
        if self.contains(Flags::B) {
            flags.push("B");
        }
        if self.contains(Flags::D) {
            flags.push("D");
        }
        if self.contains(Flags::I) {
            flags.push("I");
        }
        if self.contains(Flags::Z) {
            flags.push("Z");
        }
        if self.contains(Flags::C) {
            flags.push("C");
        }

        write!(f, "NV1BDIZC: {}", flags.join(""))
    }
}

#[derive(Clone)]
pub struct Cpu {
    pub sp: u8,
    pub pc: u16,
    pub flags: Flags,
    pub a: u8,
    pub x: u8,
    pub y: u8,
}

impl Cpu {
    pub fn new() -> Self {
        Self {
            sp: 0xfd,
            pc: 0xfffc,
            flags: Flags::from_bits(0b00000100).unwrap(),
            a: 0,
            x: 0,
            y: 0,
        }
    }

    pub fn step(&mut self) {}
}
