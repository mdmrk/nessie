use std::sync::RwLock;

use crate::{cpu::Cpu, emu::Emu};

pub struct DebugState {
    pub cpu: RwLock<Cpu>,
}

impl DebugState {
    pub fn new() -> Self {
        Self {
            cpu: RwLock::new(Cpu { a: 0 }),
        }
    }

    pub fn update(&self, emu: &Emu) {
        if let Ok(mut cpu) = self.cpu.write() {
            *cpu = emu.cpu.clone();
        }
    }
}
