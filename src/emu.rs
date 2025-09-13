use crate::cpu;

pub struct Emu {
    pub cpu: cpu::Cpu,
}

impl Emu {
    pub fn new() -> Self {
        Self {
            cpu: cpu::Cpu::new(),
        }
    }
}
