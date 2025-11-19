use std::sync::RwLock;

use crate::{cart::Header, cpu::Cpu, emu::Emu, ppu::Ppu};

pub const ROWS_TO_SHOW: usize = 7;
pub const BYTES_PER_ROW: usize = 0x10;
pub const MEM_BLOCK_SIZE: usize = ROWS_TO_SHOW * BYTES_PER_ROW;

pub struct DebugState {
    pub cpu: RwLock<Cpu>,
    pub ppu: RwLock<Ppu>,
    pub cart_header: RwLock<Option<Header>>,
    pub mem_chunk: RwLock<[u8; MEM_BLOCK_SIZE]>,
    pub stack: RwLock<[u8; 0x100]>,
}

macro_rules! profile {
    ($name:expr) => {
        #[cfg(all(debug_assertions, not(target_arch = "wasm32")))]
        puffin::profile_scope!($name);
    };
}
pub(crate) use profile;

impl Default for DebugState {
    fn default() -> Self {
        Self {
            cpu: Default::default(),
            ppu: Default::default(),
            cart_header: Default::default(),
            mem_chunk: RwLock::new([0; MEM_BLOCK_SIZE]),
            stack: RwLock::new([0; 0x100]),
        }
    }
}

impl DebugState {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn update(&self, emu: &mut Emu) {
        if let Ok(mut cpu) = self.cpu.write() {
            *cpu = emu.cpu.clone();
        }

        if let Ok(mut ppu) = self.ppu.write() {
            *ppu = emu.bus.ppu.clone();
        }

        if let Ok(mut cart_header) = self.cart_header.write() {
            *cart_header = emu.bus.cart.as_ref().map(|cart| cart.header.clone())
        }
        if let Ok(mut mem_chunk) = self.mem_chunk.write() {
            mem_chunk.copy_from_slice(
                &emu.bus
                    .read_only_range(emu.mem_chunk_addr as u16, MEM_BLOCK_SIZE as u16)
                    [..MEM_BLOCK_SIZE],
            );
        }
        if let Ok(mut stack) = self.stack.write() {
            stack.copy_from_slice(&emu.bus.read_only_range(0x100, 0x100)[..0x100]);
        }
    }
}
