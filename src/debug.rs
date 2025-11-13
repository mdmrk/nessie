use std::sync::RwLock;

use crate::{cart::Header, cpu::Cpu, emu::Emu, ppu::Ppu};

#[derive(Default)]
pub struct DebugState {
    pub cpu: RwLock<Cpu>,
    pub ppu: RwLock<Ppu>,
    pub cart_header: RwLock<Option<Header>>,
    pub cpu_log: RwLock<String>,
    pub mem_chunk: RwLock<Vec<u8>>,
    pub stack: RwLock<Vec<u8>>,
}

impl DebugState {
    pub fn new() -> Self {
        Self {
            cpu: RwLock::new(Cpu::new()),
            ppu: RwLock::new(Ppu::new()),
            cart_header: RwLock::new(None),
            cpu_log: RwLock::new("".into()),
            mem_chunk: RwLock::new(vec![0; 7 * 16]),
            stack: RwLock::new(vec![0; 0x100]),
        }
    }

    pub fn update(&self, emu: &mut Emu) {
        if let Ok(mut cpu) = self.cpu.write() {
            *cpu = emu.cpu.clone();
        }
        if let Ok(mut ppu) = self.ppu.write() {
            *ppu = emu.ppu.clone();
        }
        if let Ok(mut cart_header) = self.cart_header.write() {
            *cart_header = emu.bus.cart.as_ref().map(|cart| cart.header.clone())
        }
        if let Ok(mut cpu_log) = self.cpu_log.write() {
            cpu_log.push_str(&emu.cpu.log);
            emu.cpu.log.clear();
        }
        if let Ok(mut mem_chunk) = self.mem_chunk.write() {
            *mem_chunk = emu.bus.read(emu.mem_chunk_addr as u16, 7 * 16);
        }
        if let Ok(mut stack) = self.stack.write() {
            *stack = emu.bus.read(0x100, 0x100);
        }
    }
}
