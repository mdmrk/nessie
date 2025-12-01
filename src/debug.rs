use crate::{
    apu::Apu,
    cart::Header,
    cpu::{Cpu, Flags},
    ppu::{Ppu, PpuCtrl, PpuMask, PpuStatus},
};

#[allow(unused_macros)]
macro_rules! profile {
    ($name:expr) => {
        #[cfg(all(debug_assertions, not(target_arch = "wasm32")))]
        puffin::profile_scope!($name);
    };
}
#[allow(unused_imports)]
pub(crate) use profile;

pub const ROWS_TO_SHOW: usize = 7;
pub const BYTES_PER_ROW: usize = 0x10;
pub const MEM_BLOCK_SIZE: usize = ROWS_TO_SHOW * BYTES_PER_ROW;

#[derive(Clone)]
pub struct DebugSnapshot {
    pub cpu: CpuSnapshot,
    pub ppu: PpuSnapshot,
    pub apu: ApuSnapshot,
    pub cart: Option<CartSnapshot>,
    pub mem_chunk: [u8; MEM_BLOCK_SIZE],
    pub stack: [u8; 0x100],
}

impl Default for DebugSnapshot {
    fn default() -> Self {
        Self {
            cpu: Default::default(),
            ppu: Default::default(),
            apu: Default::default(),
            cart: None,
            mem_chunk: [0; MEM_BLOCK_SIZE],
            stack: [0; 0x100],
        }
    }
}

#[derive(Default, Clone)]
pub struct CpuSnapshot {
    pub sp: u8,
    pub pc: u16,
    pub a: u8,
    pub x: u8,
    pub y: u8,
    pub p: u8,
    pub cycles: u64,
    pub flags_n: bool,
    pub flags_v: bool,
    pub flags_b: bool,
    pub flags_d: bool,
    pub flags_i: bool,
    pub flags_z: bool,
    pub flags_c: bool,
    pub log: Option<String>,
}

#[derive(Clone)]
pub struct PpuSnapshot {
    pub dot: u16,
    pub scanline: u16,
    pub frame: u64,
    pub ctrl: PpuCtrl,
    pub mask: PpuMask,
    pub status: PpuStatus,
    pub oam_addr: u8,
    pub v: u16,
    pub t: u16,
    pub x: u8,
    pub w: bool,
    pub palette: [u8; 32],
    pub oam: [u8; 256],
}

impl Default for PpuSnapshot {
    fn default() -> Self {
        Self {
            dot: Default::default(),
            scanline: Default::default(),
            frame: Default::default(),
            ctrl: Default::default(),
            mask: Default::default(),
            status: Default::default(),
            oam_addr: Default::default(),
            v: Default::default(),
            t: Default::default(),
            x: Default::default(),
            w: Default::default(),
            palette: Default::default(),
            oam: [0; 256],
        }
    }
}

#[derive(Default, Clone)]
pub struct ApuSnapshot {
    pub pulse1_enabled: bool,
    pub pulse1_period: u16,
    pub pulse1_vol: u8,
    pub pulse1_duty: u8,

    pub pulse2_enabled: bool,
    pub pulse2_period: u16,
    pub pulse2_vol: u8,

    pub tri_enabled: bool,
    pub tri_linear: u8,

    pub noise_enabled: bool,
    pub noise_mode: bool,

    pub dmc_enabled: bool,
    pub dmc_len: u16,
    pub dmc_irq: bool,
    pub frame_mode: bool,
    pub frame_irq: bool,
}

#[derive(Default, Clone)]
pub struct CartSnapshot {
    pub magic: [u8; 4],
    pub has_trainer: bool,
    pub prg_rom_size: usize,
    pub chr_rom_size: usize,
    pub mapper_number: u8,
}

impl DebugSnapshot {
    pub fn new(
        cpu: &Cpu,
        ppu: &Ppu,
        apu: &Apu,
        cart: Option<&Header>,
        memory: &[u8],
        stack: &[u8],
    ) -> Self {
        let mut mem_chunk = [0u8; MEM_BLOCK_SIZE];
        let len = memory.len().min(MEM_BLOCK_SIZE);
        mem_chunk[..len].copy_from_slice(&memory[..len]);

        let mut stack_chunk = [0u8; 0x100];
        let len = stack.len().min(0x100);
        stack_chunk[..len].copy_from_slice(&stack[..len]);

        let log_string = cpu.log.as_ref().map(|l| l.iter().collect::<String>());

        Self {
            cpu: CpuSnapshot {
                sp: cpu.sp,
                pc: cpu.pc,
                a: cpu.a,
                x: cpu.x,
                y: cpu.y,
                p: cpu.p.bits(),
                cycles: cpu.cycle_count as u64,
                flags_n: cpu.p.contains(Flags::N),
                flags_v: cpu.p.contains(Flags::V),
                flags_b: cpu.p.contains(Flags::B),
                flags_d: cpu.p.contains(Flags::D),
                flags_i: cpu.p.contains(Flags::I),
                flags_z: cpu.p.contains(Flags::Z),
                flags_c: cpu.p.contains(Flags::C),
                log: log_string,
            },
            ppu: PpuSnapshot {
                dot: ppu.dot,
                scanline: ppu.scanline,
                frame: ppu.frame,
                ctrl: ppu.ctrl,
                mask: ppu.mask,
                status: ppu.status,
                oam_addr: ppu.oam_addr,
                v: ppu.v,
                t: ppu.t,
                x: ppu.x,
                w: ppu.w,
                palette: ppu.palette,
                oam: ppu.oam,
            },
            apu: ApuSnapshot {
                pulse1_enabled: apu.pulse1.enabled,
                pulse1_period: apu.pulse1.timer_period,
                pulse1_vol: if apu.pulse1.envelope.constant_volume {
                    apu.pulse1.envelope.divider_period
                } else {
                    apu.pulse1.envelope.decay_count
                },
                pulse1_duty: apu.pulse1.duty_mode,
                pulse2_enabled: apu.pulse2.enabled,
                pulse2_period: apu.pulse2.timer_period,
                pulse2_vol: if apu.pulse2.envelope.constant_volume {
                    apu.pulse2.envelope.divider_period
                } else {
                    apu.pulse2.envelope.decay_count
                },
                tri_enabled: apu.triangle.enabled,
                tri_linear: apu.triangle.linear_counter,
                noise_enabled: apu.noise.enabled,
                noise_mode: apu.noise.mode,
                dmc_enabled: apu.dmc.enabled,
                dmc_len: apu.dmc.current_length,
                dmc_irq: apu.dmc.irq_pending,
                frame_mode: apu.frame_mode,
                frame_irq: apu.frame_irq_pending,
            },
            cart: cart.map(|h| CartSnapshot {
                magic: h.magic,
                has_trainer: h.flags6.has_trainer(),
                prg_rom_size: h.prg_rom_size as usize,
                chr_rom_size: h.chr_rom_size as usize,
                mapper_number: h.mapper_number(),
            }),
            mem_chunk,
            stack: stack_chunk,
        }
    }
}
