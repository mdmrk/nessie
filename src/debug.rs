use std::sync::RwLock;

use log::warn;

use crate::{bus::Bus, cart::Header, cpu::Cpu, emu::Emu};

#[derive(Default)]
pub struct DebugLog {
    pub log: Vec<String>,
    pub line: usize,
}

impl DebugLog {
    pub fn new(logfile: &String) -> Self {
        let log = String::from_utf8(std::fs::read(logfile).unwrap_or_default())
            .unwrap()
            .split("\n")
            .map(|s| s.to_string())
            .collect();
        Self { log, line: 0 }
    }

    pub fn compare(&mut self, debug_str: &str) -> bool {
        let line = self.line;
        self.line += 1;
        self.log[line].trim() == debug_str.trim()
    }
}

#[derive(Default)]
pub struct DebugState {
    pub cpu: RwLock<Cpu>,
    pub bus: RwLock<Bus>,
    pub cart_header: RwLock<Option<Header>>,
    pub cpu_log: RwLock<String>,
}

impl DebugState {
    pub fn new() -> Self {
        Self {
            cpu: RwLock::new(Cpu::new()),
            bus: RwLock::new(Bus::new()),
            cart_header: RwLock::new(None),
            cpu_log: RwLock::new("".into()),
        }
    }

    pub fn update(&self, emu: &mut Emu) {
        if let Ok(mut cpu) = self.cpu.write() {
            *cpu = emu.cpu.clone();
        }
        if let Ok(mut bus) = self.bus.write() {
            *bus = emu.bus.clone();
        }
        if let Ok(mut cart_header) = self.cart_header.write() {
            *cart_header = emu.cart.as_ref().map(|cart| cart.header.clone())
        }
        if let Ok(mut cpu_log) = self.cpu_log.write() {
            cpu_log.push_str(&emu.cpu.log);
            emu.cpu.log.clear();
        }
    }
}
