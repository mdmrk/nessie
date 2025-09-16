use std::sync::RwLock;

use crate::{bus::Bus, cpu::Cpu, emu::Emu};

pub struct DebugState {
    pub cpu: RwLock<Cpu>,
    pub bus: RwLock<Bus>,
}

impl DebugState {
    pub fn new() -> Self {
        Self {
            cpu: RwLock::new(Cpu::new()),
            bus: RwLock::new(Bus::new()),
        }
    }

    pub fn update(&self, emu: &Emu) {
        if let Ok(mut cpu) = self.cpu.write() {
            *cpu = emu.cpu.clone();
        }
        if let Ok(mut bus) = self.bus.write() {
            *bus = emu.bus.clone();
        }
    }
}
