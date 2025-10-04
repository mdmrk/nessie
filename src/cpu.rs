use core::fmt;

use bitflags::bitflags;
use log::warn;
use phf::phf_map;

use crate::{bus::Bus, debug::DebugLog, ppu::Ppu};

#[derive(Debug)]
pub enum OperandValue {
    Implicid,
    Address(u16, bool),
    Value(u8),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AddressingMode {
    Implicid,
    Accumulator,
    Immediate,
    ZeroPage,
    ZeroPageX,
    ZeroPageY,
    Absolute,
    AbsoluteX,
    AbsoluteY,
    Indirect,
    IndirectX,
    IndirectY,
    Relative,
}

impl AddressingMode {
    pub fn resolve(&self, cpu: &Cpu, bus: &Bus, operands: &[u8]) -> OperandValue {
        match self {
            AddressingMode::Implicid | AddressingMode::Accumulator => OperandValue::Implicid,
            AddressingMode::Immediate => OperandValue::Value(operands[0]),
            AddressingMode::ZeroPage => OperandValue::Address(operands[0] as u16, false),
            AddressingMode::ZeroPageX => {
                let addr = operands[0].wrapping_add(cpu.x) as u16;
                OperandValue::Address(addr, false)
            }
            AddressingMode::ZeroPageY => {
                let addr = operands[0].wrapping_add(cpu.y) as u16;
                OperandValue::Address(addr, false)
            }
            AddressingMode::Absolute => {
                let addr = u16::from_le_bytes([operands[0], operands[1]]);
                OperandValue::Address(addr, false)
            }
            AddressingMode::AbsoluteX => {
                let base = u16::from_le_bytes([operands[0], operands[1]]);
                let addr = base.wrapping_add(cpu.x as u16);
                let page_crossed = (base & 0xFF00) != (addr & 0xFF00);
                OperandValue::Address(addr, page_crossed)
            }
            AddressingMode::AbsoluteY => {
                let base = u16::from_le_bytes([operands[0], operands[1]]);
                let addr = base.wrapping_add(cpu.y as u16);
                let page_crossed = (base & 0xFF00) != (addr & 0xFF00);
                OperandValue::Address(addr, page_crossed)
            }
            AddressingMode::Indirect => {
                let ptr = u16::from_le_bytes([operands[0], operands[1]]);
                let lo = bus.read_byte(ptr as usize);
                let hi = bus.read_byte(((ptr & 0xFF00) | ((ptr + 1) & 0x00FF)) as usize);
                let addr = u16::from_le_bytes([lo, hi]);
                OperandValue::Address(addr, false)
            }
            AddressingMode::IndirectX => {
                let ptr = operands[0].wrapping_add(cpu.x);
                let lo = bus.read_byte(ptr as usize);
                let hi = bus.read_byte(ptr.wrapping_add(1) as usize);
                let addr = u16::from_le_bytes([lo, hi]);
                OperandValue::Address(addr, false)
            }
            AddressingMode::IndirectY => {
                let ptr = operands[0];
                let lo = bus.read_byte(ptr as usize);
                let hi = bus.read_byte(ptr.wrapping_add(1) as usize);
                let base = u16::from_le_bytes([lo, hi]);
                let addr = base.wrapping_add(cpu.y as u16);
                let page_crossed = (base & 0xFF00) != (addr & 0xFF00);
                OperandValue::Address(addr, page_crossed)
            }
            AddressingMode::Relative => {
                let offset = operands[0] as i8;
                let addr = cpu.pc.wrapping_add_signed(offset as i16);
                let page_crossed = (cpu.pc & 0xFF00) != (addr & 0xFF00);
                OperandValue::Address(addr, page_crossed)
            }
        }
    }

    pub fn operand_bytes(&self) -> u8 {
        match self {
            AddressingMode::Implicid | AddressingMode::Accumulator => 0,
            AddressingMode::Immediate
            | AddressingMode::ZeroPage
            | AddressingMode::ZeroPageX
            | AddressingMode::ZeroPageY
            | AddressingMode::IndirectX
            | AddressingMode::IndirectY
            | AddressingMode::Relative => 1,
            AddressingMode::Absolute
            | AddressingMode::AbsoluteX
            | AddressingMode::AbsoluteY
            | AddressingMode::Indirect => 2,
        }
    }
}

#[derive(Debug)]
pub enum OpMnemonic {
    LDA,
    STA,
    LDX,
    STX,
    LDY,
    STY,
    TAX,
    TXA,
    TAY,
    TYA,
    ADC,
    SBC,
    INC,
    DEC,
    INX,
    DEX,
    INY,
    DEY,
    ASL,
    LSR,
    ROL,
    ROR,
    AND,
    ORA,
    EOR,
    BIT,
    CMP,
    CPX,
    CPY,
    BCC,
    BCS,
    BEQ,
    BNE,
    BPL,
    BMI,
    BVC,
    BVS,
    JMP,
    JSR,
    RTS,
    BRK,
    RTI,
    PHA,
    PLA,
    PHP,
    PLP,
    TXS,
    TSX,
    CLC,
    SEC,
    CLI,
    SEI,
    CLD,
    SED,
    CLV,
    NOP,
}

impl fmt::Display for OpMnemonic {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub struct Op {
    pub mnemonic: OpMnemonic,
    pub mode: AddressingMode,
    pub base_cycles: usize,
    pub execute: fn(&mut Cpu, &mut Bus, AddressingMode, &[u8]) -> u8,
}

macro_rules! op {
    ($mnemonic:expr, $mode:expr, $base_cycles:expr, $execute:expr) => {
        Op {
            mnemonic: $mnemonic,
            mode: $mode,
            base_cycles: $base_cycles,
            execute: $execute,
        }
    };
}

static OPCODES: phf::Map<u8, Op> = phf_map! {
    0xA9u8 => op!(OpMnemonic::LDA, AddressingMode::Immediate,  2, Cpu::lda),
    0x85u8 => op!(OpMnemonic::STA, AddressingMode::ZeroPage,  3, Cpu::sta),
    0xA2u8 => op!(OpMnemonic::LDX, AddressingMode::Immediate,  2, Cpu::ldx),
    0x86u8 => op!(OpMnemonic::STX, AddressingMode::ZeroPage,  3, Cpu::stx),
    // 0xu8 => op!(OpMnemonic::LDY, AddressingMode::Immediate,  0, Cpu::xxx),
    // 0xu8 => op!(OpMnemonic::STY, AddressingMode::Immediate,  0, Cpu::xxx),
    // 0xu8 => op!(OpMnemonic::TAX, AddressingMode::Immediate,  0, Cpu::xxx),
    // 0xu8 => op!(OpMnemonic::TXA, AddressingMode::Immediate,  0, Cpu::xxx),
    // 0xu8 => op!(OpMnemonic::TAY, AddressingMode::Immediate,  0, Cpu::xxx),
    // 0xu8 => op!(OpMnemonic::TYA, AddressingMode::Immediate,  0, Cpu::xxx),
    // 0xu8 => op!(OpMnemonic::ADC, AddressingMode::Immediate,  0, Cpu::xxx),
    // 0xu8 => op!(OpMnemonic::SBC, AddressingMode::Immediate,  0, Cpu::xxx),
    // 0xu8 => op!(OpMnemonic::INC, AddressingMode::Immediate,  0, Cpu::xxx),
    // 0xu8 => op!(OpMnemonic::DEC, AddressingMode::Immediate,  0, Cpu::xxx),
    // 0xu8 => op!(OpMnemonic::INX, AddressingMode::Immediate,  0, Cpu::xxx),
    // 0xu8 => op!(OpMnemonic::DEX, AddressingMode::Immediate,  0, Cpu::xxx),
    // 0xu8 => op!(OpMnemonic::INY, AddressingMode::Immediate,  0, Cpu::xxx),
    // 0xu8 => op!(OpMnemonic::DEY, AddressingMode::Immediate,  0, Cpu::xxx),
    // 0xu8 => op!(OpMnemonic::ASL, AddressingMode::Immediate,  0, Cpu::xxx),
    // 0xu8 => op!(OpMnemonic::LSR, AddressingMode::Immediate,  0, Cpu::xxx),
    // 0xu8 => op!(OpMnemonic::ROL, AddressingMode::Immediate,  0, Cpu::xxx),
    // 0xu8 => op!(OpMnemonic::ROR, AddressingMode::Immediate,  0, Cpu::xxx),
    0x29u8 => op!(OpMnemonic::AND, AddressingMode::Immediate,  2, Cpu::and),
    0x09u8 => op!(OpMnemonic::ORA, AddressingMode::Immediate,  2, Cpu::ora),
    // 0xu8 => op!(OpMnemonic::EOR, AddressingMode::Immediate,  0, Cpu::xxx),
    0x24u8 => op!(OpMnemonic::BIT, AddressingMode::Immediate,  3, Cpu::bit),
    // 0xu8 => op!(OpMnemonic::CMP, AddressingMode::Immediate,  0, Cpu::xxx),
    // 0xu8 => op!(OpMnemonic::CPX, AddressingMode::Immediate,  0, Cpu::xxx),
    // 0xu8 => op!(OpMnemonic::CPY, AddressingMode::Immediate,  0, Cpu::xxx),
    0x90u8 => op!(OpMnemonic::BCC, AddressingMode::Relative,  2, Cpu::bcc),
    0xB0u8 => op!(OpMnemonic::BCS, AddressingMode::Relative,  2, Cpu::bcs),
    0xF0u8 => op!(OpMnemonic::BEQ, AddressingMode::Relative,  2, Cpu::beq),
    0xD0u8 => op!(OpMnemonic::BNE, AddressingMode::Relative,  2, Cpu::bne),
    0x10u8 => op!(OpMnemonic::BPL, AddressingMode::Relative,  2, Cpu::bpl),
    0x30u8 => op!(OpMnemonic::BMI, AddressingMode::Relative,  2, Cpu::bmi),
    0x50u8 => op!(OpMnemonic::BVC, AddressingMode::Immediate,  2, Cpu::bvc),
    0x70u8 => op!(OpMnemonic::BVS, AddressingMode::Immediate,  2, Cpu::bvs),
    0x4Cu8 => op!(OpMnemonic::JMP, AddressingMode::Absolute,  3, Cpu::jmp),
    0x20u8 => op!(OpMnemonic::JSR, AddressingMode::Absolute,  6, Cpu::jsr),
    // 0xu8 => op!(OpMnemonic::RTS, AddressingMode::Immediate,  0, Cpu::xxx),
    // 0xu8 => op!(OpMnemonic::BRK, AddressingMode::Immediate,  0, Cpu::xxx),
    // 0xu8 => op!(OpMnemonic::RTI, AddressingMode::Immediate,  0, Cpu::xxx),
    0x48u8 => op!(OpMnemonic::PHA, AddressingMode::Implicid,  3, Cpu::pha),
    0x68u8 => op!(OpMnemonic::PLA, AddressingMode::Implicid,  4, Cpu::pla),
    0x08u8 => op!(OpMnemonic::PHP, AddressingMode::Implicid,  3, Cpu::php),
    0x28u8 => op!(OpMnemonic::PLP, AddressingMode::Implicid,  4, Cpu::plp),
    // 0xu8 => op!(OpMnemonic::TXS, AddressingMode::Immediate,  0, Cpu::xxx),
    // 0xu8 => op!(OpMnemonic::TSX, AddressingMode::Immediate,  0, Cpu::xxx),
    0x18u8 => op!(OpMnemonic::CLC, AddressingMode::Implicid,  2, Cpu::clc),
    0x38u8 => op!(OpMnemonic::SEC, AddressingMode::Implicid,  2, Cpu::sec),
    // 0xu8 => op!(OpMnemonic::CLI, AddressingMode::Immediate,  0, Cpu::xxx),
    // 0xu8 => op!(OpMnemonic::SEI, AddressingMode::Immediate,  0, Cpu::xxx),
    // 0xu8 => op!(OpMnemonic::CLD, AddressingMode::Immediate,  0, Cpu::xxx),
    // 0xu8 => op!(OpMnemonic::SED, AddressingMode::Immediate,  0, Cpu::xxx),
    // 0xu8 => op!(OpMnemonic::CLV, AddressingMode::Immediate,  0, Cpu::xxx),
    0xEAu8 => op!(OpMnemonic::NOP, AddressingMode::Implicid,  2, Cpu::nop),
};

bitflags! {
    #[derive(Debug, Clone)]
    pub struct Flags: u8 {
        const N = 1 << 7;
        const V = 1 << 6;
        const _1 = 1 << 5;
        const B = 1 << 4;
        const D = 1 << 3;
        const I = 1 << 2;
        const Z = 1 << 1;
        const C = 1 << 0;
    }
}

#[derive(Clone)]
pub struct Cpu {
    pub sp: usize,
    pub pc: u16,
    pub p: Flags,
    pub a: u8,
    pub x: u8,
    pub y: u8,
    pub cycle_count: usize,
    pub log: String,
}

impl Default for Cpu {
    fn default() -> Self {
        Self::new()
    }
}

impl Cpu {
    pub fn new() -> Self {
        Self {
            sp: 0xfd,
            pc: 0,
            p: Flags::I | Flags::_1,
            a: 0,
            x: 0,
            y: 0,
            cycle_count: 7, // FIXME: do proper init state / reset
            log: "".into(),
        }
    }

    fn fetch(&self, bus: &Bus) -> u8 {
        bus.read_byte(self.pc as usize)
    }

    fn decode(&self, opcode: u8) -> Option<&'static Op> {
        OPCODES.get(&opcode)
    }

    fn execute(
        &mut self,
        bus: &mut Bus,
        ppu: &mut Ppu,
        op: &Op,
        opcode: u8,
        debug_log: &mut Option<DebugLog>,
    ) -> bool {
        let operand_bytes = op.mode.operand_bytes();
        let operands = bus.read(self.pc + 1, operand_bytes as u16).to_vec(); // FIXME: should not clone

        let debug_str = format!(
            "{:04X}  {:02X} {:6} {} A:{:02X} X:{:02X} Y:{:02X} P:{:02X} SP:{:02X} PPU:{:3},{:3} CYC:{}\n",
            self.pc,
            opcode,
            operands
                .iter()
                .map(|c| format!("{:02X}", c))
                .collect::<Vec<String>>()
                .join(" "),
            op.mnemonic,
            self.a,
            self.x,
            self.y,
            self.p.bits(),
            self.sp,
            ppu.scanline,
            ppu.h_pixel,
            self.cycle_count
        );

        self.pc += 1 + operand_bytes as u16;
        let extra_cycles = (op.execute)(self, bus, op.mode, &operands);
        let total_cycles = op.base_cycles + extra_cycles as usize;
        self.cycle_count += total_cycles;
        ppu.step(total_cycles);

        self.log.push_str(&debug_str);
        if let Some(debug_log) = debug_log {
            let ok = debug_log.compare(&debug_str);
            if !ok {
                let mut log = debug_log.log[debug_log.line - 1].clone();
                log.push_str("     [ACTUAL LOG]");
                self.log.push_str(&log);
            }
            ok
        } else {
            true
        }
    }

    pub fn step(&mut self, bus: &mut Bus, ppu: &mut Ppu, debug_log: &mut Option<DebugLog>) -> bool {
        let opcode = self.fetch(bus);
        let op = self.decode(opcode);

        match op {
            Some(op) => self.execute(bus, ppu, op, opcode, debug_log),
            None => {
                warn!("Unknown opcode: 0x{:02X}", opcode);
                true
            }
        }
    }

    fn update_nz(&mut self, value: u8) {
        self.p.set(Flags::Z, value == 0);
        self.p.set(Flags::N, (value >> 7) & 1 == 1);
    }

    fn push_stack(&mut self, bus: &mut Bus, value: u8) {
        bus.write_byte(0x100 + self.sp, value);
        self.sp = self.sp.wrapping_sub(1);
    }

    fn pop_stack(&mut self, bus: &Bus) -> u8 {
        self.sp = self.sp.wrapping_add(1);
        bus.read_byte(0x100 + self.sp)
    }

    fn read_operand(&self, bus: &Bus, mode: AddressingMode, operands: &[u8]) -> (u8, bool) {
        match mode.resolve(self, bus, operands) {
            OperandValue::Value(v) => (v, false),
            OperandValue::Address(addr, crossed) => (bus.read_byte(addr as usize), crossed),
            OperandValue::Implicid => (self.a, false),
        }
    }

    fn write_operand(&mut self, bus: &mut Bus, mode: AddressingMode, operands: &[u8], value: u8) {
        match mode.resolve(self, bus, operands) {
            OperandValue::Address(addr, _) => bus.write_byte(addr as usize, value),
            OperandValue::Implicid => self.a = value,
            _ => panic!("Cannot write to this addressing mode"),
        }
    }

    fn lda(cpu: &mut Cpu, bus: &mut Bus, mode: AddressingMode, operands: &[u8]) -> u8 {
        let (value, page_crossed) = cpu.read_operand(bus, mode, operands);
        cpu.a = value;
        cpu.update_nz(cpu.a);
        if page_crossed { 1 } else { 0 }
    }

    fn sta(cpu: &mut Cpu, bus: &mut Bus, mode: AddressingMode, operands: &[u8]) -> u8 {
        cpu.write_operand(bus, mode, operands, cpu.a);
        0
    }

    fn ldx(cpu: &mut Cpu, bus: &mut Bus, mode: AddressingMode, operands: &[u8]) -> u8 {
        let (value, page_crossed) = cpu.read_operand(bus, mode, operands);
        cpu.x = value;
        cpu.update_nz(cpu.x);
        if page_crossed { 1 } else { 0 }
    }

    fn stx(cpu: &mut Cpu, bus: &mut Bus, mode: AddressingMode, operands: &[u8]) -> u8 {
        cpu.write_operand(bus, mode, operands, cpu.x);
        0
    }

    // fn ldy(cpu: &mut Cpu, bus: &mut Bus, mode: AddressingMode, operands: &[u8]) -> u8 {}

    // fn sty(cpu: &mut Cpu, bus: &mut Bus, mode: AddressingMode, operands: &[u8]) -> u8 {}

    // fn tax(cpu: &mut Cpu, bus: &mut Bus, mode: AddressingMode, operands: &[u8]) -> u8 {}

    // fn txa(cpu: &mut Cpu, bus: &mut Bus, mode: AddressingMode, operands: &[u8]) -> u8 {}

    // fn tay(cpu: &mut Cpu, bus: &mut Bus, mode: AddressingMode, operands: &[u8]) -> u8 {}

    // fn tya(cpu: &mut Cpu, bus: &mut Bus, mode: AddressingMode, operands: &[u8]) -> u8 {}

    // fn adc(cpu: &mut Cpu, bus: &mut Bus, mode: AddressingMode, operands: &[u8]) -> u8 {}

    // fn sbc(cpu: &mut Cpu, bus: &mut Bus, mode: AddressingMode, operands: &[u8]) -> u8 {}

    // fn inc(cpu: &mut Cpu, bus: &mut Bus, mode: AddressingMode, operands: &[u8]) -> u8 {}

    // fn dec(cpu: &mut Cpu, bus: &mut Bus, mode: AddressingMode, operands: &[u8]) -> u8 {}

    // fn inx(cpu: &mut Cpu, bus: &mut Bus, mode: AddressingMode, operands: &[u8]) -> u8 {}

    // fn dex(cpu: &mut Cpu, bus: &mut Bus, mode: AddressingMode, operands: &[u8]) -> u8 {}

    // fn iny(cpu: &mut Cpu, bus: &mut Bus, mode: AddressingMode, operands: &[u8]) -> u8 {}

    // fn dey(cpu: &mut Cpu, bus: &mut Bus, mode: AddressingMode, operands: &[u8]) -> u8 {}

    // fn asl(cpu: &mut Cpu, bus: &mut Bus, mode: AddressingMode, operands: &[u8]) -> u8 {}

    // fn lsr(cpu: &mut Cpu, bus: &mut Bus, mode: AddressingMode, operands: &[u8]) -> u8 {}

    // fn rol(cpu: &mut Cpu, bus: &mut Bus, mode: AddressingMode, operands: &[u8]) -> u8 {}

    // fn ror(cpu: &mut Cpu, bus: &mut Bus, mode: AddressingMode, operands: &[u8]) -> u8 {}

    fn and(cpu: &mut Cpu, bus: &mut Bus, mode: AddressingMode, operands: &[u8]) -> u8 {
        let (value, page_crossed) = cpu.read_operand(bus, mode, operands);
        cpu.a &= value;
        cpu.update_nz(cpu.a);
        match mode {
            AddressingMode::AbsoluteX | AddressingMode::AbsoluteY => {
                if page_crossed {
                    2
                } else {
                    1
                }
            }
            AddressingMode::IndirectY => {
                if page_crossed {
                    4
                } else {
                    3
                }
            }
            _ => 0,
        }
    }

    fn ora(cpu: &mut Cpu, bus: &mut Bus, mode: AddressingMode, operands: &[u8]) -> u8 {
        let (value, page_crossed) = cpu.read_operand(bus, mode, operands);
        cpu.a |= value;
        cpu.update_nz(cpu.a);
        match mode {
            AddressingMode::AbsoluteX | AddressingMode::AbsoluteY => {
                if page_crossed {
                    2
                } else {
                    1
                }
            }
            AddressingMode::IndirectY => {
                if page_crossed {
                    4
                } else {
                    3
                }
            }
            _ => 0,
        }
    }

    // fn eor(cpu: &mut Cpu, bus: &mut Bus, mode: AddressingMode, operands: &[u8]) -> u8 {}

    fn bit(cpu: &mut Cpu, bus: &mut Bus, mode: AddressingMode, operands: &[u8]) -> u8 {
        let (value, _) = cpu.read_operand(bus, mode, operands);
        let result = value & cpu.a;
        cpu.p.set(Flags::Z, result == 0);
        cpu.p.set(Flags::V, result & 0b0100_0000 == 0);
        cpu.p.set(Flags::N, result & 0b1000_0000 == 0);
        0
    }

    // fn cmp(cpu: &mut Cpu, bus: &mut Bus, mode: AddressingMode, operands: &[u8]) -> u8 {}

    // fn cpx(cpu: &mut Cpu, bus: &mut Bus, mode: AddressingMode, operands: &[u8]) -> u8 {}

    // fn cpy(cpu: &mut Cpu, bus: &mut Bus, mode: AddressingMode, operands: &[u8]) -> u8 {}

    fn bcc(cpu: &mut Cpu, bus: &mut Bus, mode: AddressingMode, operands: &[u8]) -> u8 {
        if !cpu.p.contains(Flags::C) {
            if let OperandValue::Address(addr, page_crossed) = mode.resolve(cpu, bus, operands) {
                cpu.pc = addr;
                if page_crossed { 2 } else { 1 }
            } else {
                0
            }
        } else {
            0
        }
    }

    fn bcs(cpu: &mut Cpu, bus: &mut Bus, mode: AddressingMode, operands: &[u8]) -> u8 {
        if cpu.p.contains(Flags::C) {
            if let OperandValue::Address(addr, page_crossed) = mode.resolve(cpu, bus, operands) {
                cpu.pc = addr;
                if page_crossed { 2 } else { 1 }
            } else {
                0
            }
        } else {
            0
        }
    }

    fn beq(cpu: &mut Cpu, bus: &mut Bus, mode: AddressingMode, operands: &[u8]) -> u8 {
        if cpu.p.contains(Flags::Z) {
            if let OperandValue::Address(addr, page_crossed) = mode.resolve(cpu, bus, operands) {
                cpu.pc = addr;
                if page_crossed { 2 } else { 1 }
            } else {
                0
            }
        } else {
            0
        }
    }

    fn bne(cpu: &mut Cpu, bus: &mut Bus, mode: AddressingMode, operands: &[u8]) -> u8 {
        if !cpu.p.contains(Flags::Z) {
            if let OperandValue::Address(addr, page_crossed) = mode.resolve(cpu, bus, operands) {
                cpu.pc = addr;
                if page_crossed { 2 } else { 1 }
            } else {
                0
            }
        } else {
            0
        }
    }

    fn bpl(cpu: &mut Cpu, bus: &mut Bus, mode: AddressingMode, operands: &[u8]) -> u8 {
        if !cpu.p.contains(Flags::N) {
            if let OperandValue::Address(addr, page_crossed) = mode.resolve(cpu, bus, operands) {
                cpu.pc = addr;
                if page_crossed { 2 } else { 1 }
            } else {
                0
            }
        } else {
            0
        }
    }

    fn bmi(cpu: &mut Cpu, bus: &mut Bus, mode: AddressingMode, operands: &[u8]) -> u8 {
        if !cpu.p.contains(Flags::N) {
            if let OperandValue::Address(addr, page_crossed) = mode.resolve(cpu, bus, operands) {
                cpu.pc = addr;
                if page_crossed { 2 } else { 1 }
            } else {
                0
            }
        } else {
            0
        }
    }

    fn bvc(cpu: &mut Cpu, bus: &mut Bus, mode: AddressingMode, operands: &[u8]) -> u8 {
        if !cpu.p.contains(Flags::V) {
            if let OperandValue::Address(addr, page_crossed) = mode.resolve(cpu, bus, operands) {
                cpu.pc = addr;
                if page_crossed { 2 } else { 1 }
            } else {
                0
            }
        } else {
            0
        }
    }

    fn bvs(cpu: &mut Cpu, bus: &mut Bus, mode: AddressingMode, operands: &[u8]) -> u8 {
        if cpu.p.contains(Flags::V) {
            if let OperandValue::Address(addr, page_crossed) = mode.resolve(cpu, bus, operands) {
                cpu.pc = addr;
                if page_crossed { 2 } else { 1 }
            } else {
                0
            }
        } else {
            0
        }
    }

    fn jmp(cpu: &mut Cpu, bus: &mut Bus, mode: AddressingMode, operands: &[u8]) -> u8 {
        if let OperandValue::Address(addr, _) = mode.resolve(cpu, bus, operands) {
            cpu.pc = addr;
        }
        0
    }

    fn jsr(cpu: &mut Cpu, bus: &mut Bus, mode: AddressingMode, operands: &[u8]) -> u8 {
        cpu.push_stack(bus, (cpu.pc >> 8) as u8);
        cpu.push_stack(bus, cpu.pc as u8);
        if let OperandValue::Address(addr, _) = mode.resolve(cpu, bus, operands) {
            cpu.pc = addr;
        }
        0
    }

    // fn rts(cpu: &mut Cpu, bus: &mut Bus, mode: AddressingMode, operands: &[u8]) -> u8 {}

    // fn brk(cpu: &mut Cpu, bus: &mut Bus, mode: AddressingMode, operands: &[u8]) -> u8 {}

    // fn rti(cpu: &mut Cpu, bus: &mut Bus, mode: AddressingMode, operands: &[u8]) -> u8 {}

    fn pha(cpu: &mut Cpu, bus: &mut Bus, _mode: AddressingMode, _operands: &[u8]) -> u8 {
        cpu.push_stack(bus, cpu.a);
        0
    }

    fn pla(cpu: &mut Cpu, bus: &mut Bus, _mode: AddressingMode, _operands: &[u8]) -> u8 {
        cpu.a = cpu.pop_stack(bus);
        cpu.update_nz(cpu.a);
        0
    }

    fn php(cpu: &mut Cpu, bus: &mut Bus, _mode: AddressingMode, _operands: &[u8]) -> u8 {
        let p = cpu.p.clone() | Flags::B;
        cpu.push_stack(bus, p.bits());
        0
    }

    fn plp(cpu: &mut Cpu, bus: &mut Bus, _mode: AddressingMode, _operands: &[u8]) -> u8 {
        let p = Flags::from_bits(cpu.pop_stack(bus)).unwrap();
        cpu.p = p;
        0
    }

    // fn txs(cpu: &mut Cpu, bus: &mut Bus, mode: AddressingMode, operands: &[u8]) -> u8 {}

    // fn tsx(cpu: &mut Cpu, bus: &mut Bus, mode: AddressingMode, operands: &[u8]) -> u8 {}

    fn clc(cpu: &mut Cpu, _bus: &mut Bus, _mode: AddressingMode, _operands: &[u8]) -> u8 {
        cpu.p.set(Flags::C, false);
        0
    }

    fn sec(cpu: &mut Cpu, _bus: &mut Bus, _mode: AddressingMode, _operands: &[u8]) -> u8 {
        cpu.p.set(Flags::C, true);
        0
    }

    // fn cli(cpu: &mut Cpu, bus: &mut Bus, mode: AddressingMode, operands: &[u8]) -> u8 {}

    // fn sei(cpu: &mut Cpu, bus: &mut Bus, mode: AddressingMode, operands: &[u8]) -> u8 {}

    // fn cld(cpu: &mut Cpu, bus: &mut Bus, mode: AddressingMode, operands: &[u8]) -> u8 {}

    // fn sed(cpu: &mut Cpu, bus: &mut Bus, mode: AddressingMode, operands: &[u8]) -> u8 {}

    // fn clv(cpu: &mut Cpu, bus: &mut Bus, mode: AddressingMode, operands: &[u8]) -> u8 {}

    fn nop(_cpu: &mut Cpu, _bus: &mut Bus, _mode: AddressingMode, _operands: &[u8]) -> u8 {
        0
    }
}
