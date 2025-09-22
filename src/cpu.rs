use bitflags::bitflags;
use log::warn;

use crate::{bus::Bus, cart::Cart};

bitflags! {
    #[derive(Debug, Clone)]
    pub struct Flags: u8 {
        const N = 1 << 7;
        const V = 1 << 6;
        const _ = 1 << 5;
        const B = 1 << 4;
        const D = 1 << 3;
        const I = 1 << 2;
        const Z = 1 << 1;
        const C = 1 << 0;
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
            flags: Flags::I,
            a: 0,
            x: 0,
            y: 0,
        }
    }

    fn fetch(&mut self, bus: &Bus, cart: &mut Cart) -> u8 {
        let pc = self.pc as usize;
        self.pc += 1;
        return bus.read_byte(pc, cart);
    }

    fn execute(&self, opcode: u8, bus: &Bus) {
        match opcode {
            _ => warn!("Invalid opcode 0x{:x}", opcode),
        }
    }

    pub fn step(&mut self, bus: &Bus, cart: &mut Cart) {
        let opcode = self.fetch(bus, cart);
        self.execute(opcode, bus);
    }
}
