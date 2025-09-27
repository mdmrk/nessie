use std::sync::RwLock;

use crate::{bus::Bus, cart::Header, cpu::Cpu, emu::Emu};

pub struct DebugState {
    pub cpu: RwLock<Cpu>,
    pub bus: RwLock<Bus>,
    pub cart_header: RwLock<Option<Header>>,
}

impl Default for DebugState {
    fn default() -> Self {
        Self::new()
    }
}

impl DebugState {
    pub fn new() -> Self {
        Self {
            cpu: RwLock::new(Cpu::new()),
            bus: RwLock::new(Bus::new()),
            cart_header: RwLock::new(None),
        }
    }

    pub fn update(&self, emu: &Emu) {
        if let Ok(mut cpu) = self.cpu.write() {
            *cpu = emu.cpu.clone();
        }
        if let Ok(mut bus) = self.bus.write() {
            *bus = emu.bus.clone();
        }
        if let Ok(mut cart_header) = self.cart_header.write() {
            *cart_header = emu.cart.as_ref().map(|cart| cart.header.clone())
        }
    }
}
