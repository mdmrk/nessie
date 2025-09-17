use std::sync::RwLock;

use crate::{bus::Bus, cart::Cart, cpu::Cpu, emu::Emu};

pub struct DebugState {
    pub cpu: RwLock<Cpu>,
    pub bus: RwLock<Bus>,
    pub cart: RwLock<Option<Cart>>,
}

impl DebugState {
    pub fn new() -> Self {
        Self {
            cpu: RwLock::new(Cpu::new()),
            bus: RwLock::new(Bus::new()),
            cart: RwLock::new(None),
        }
    }

    pub fn update(&self, emu: &Emu) {
        if let Ok(mut cpu) = self.cpu.write() {
            *cpu = emu.cpu.clone();
        }
        if let Ok(mut bus) = self.bus.write() {
            *bus = emu.bus.clone();
        }
        if let Ok(mut cart) = self.cart.write() {
            *cart = emu.cart.clone();
        }
    }
}
