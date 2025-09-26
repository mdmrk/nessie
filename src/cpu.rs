use core::fmt;

use bitflags::bitflags;
use log::{debug, error, warn};

use crate::bus::Bus;

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
    pub bytes: u16,
    pub cycles: &'static [usize],
}

macro_rules! op {
    ($mnemonic:expr, $bytes:expr, $cycles:expr) => {
        Some(Op {
            mnemonic: $mnemonic,
            bytes: $bytes,
            cycles: $cycles,
        })
    };
}

impl Op {
    pub fn from_opcode(opcode: u8) -> Option<Self> {
        match opcode {
            0xA9 => op!(OpMnemonic::LDA, 2, &[2]),
            // 0x00 => op!(OpMnemonic::STA, 0, &[0]),
            0xA2 => op!(OpMnemonic::LDX, 2, &[2]),
            0x86 => op!(OpMnemonic::STX, 2, &[3]),
            // 0x00 => op!(OpMnemonic::LDY, 0, &[0]),
            // 0x00 => op!(OpMnemonic::STY, 0, &[0]),
            // 0x00 => op!(OpMnemonic::TAX, 0, &[0]),
            // 0x00 => op!(OpMnemonic::TXA, 0, &[0]),
            // 0x00 => op!(OpMnemonic::TAY, 0, &[0]),
            // 0x00 => op!(OpMnemonic::TYA, 0, &[0]),
            // 0x00 => op!(OpMnemonic::ADC, 0, &[0]),
            // 0x00 => op!(OpMnemonic::SBC, 0, &[0]),
            // 0x00 => op!(OpMnemonic::INC, 0, &[0]),
            // 0x00 => op!(OpMnemonic::DEC, 0, &[0]),
            // 0x00 => op!(OpMnemonic::INX, 0, &[0]),
            // 0x00 => op!(OpMnemonic::DEX, 0, &[0]),
            // 0x00 => op!(OpMnemonic::INY, 0, &[0]),
            // 0x00 => op!(OpMnemonic::DEY, 0, &[0]),
            // 0x00 => op!(OpMnemonic::ASL, 0, &[0]),
            // 0x00 => op!(OpMnemonic::LSR, 0, &[0]),
            // 0x00 => op!(OpMnemonic::ROL, 0, &[0]),
            // 0x00 => op!(OpMnemonic::ROR, 0, &[0]),
            // 0x00 => op!(OpMnemonic::AND, 0, &[0]),
            // 0x00 => op!(OpMnemonic::ORA, 0, &[0]),
            // 0x00 => op!(OpMnemonic::EOR, 0, &[0]),
            // 0x00 => op!(OpMnemonic::BIT, 0, &[0]),
            // 0x00 => op!(OpMnemonic::CMP, 0, &[0]),
            // 0x00 => op!(OpMnemonic::CPX, 0, &[0]),
            // 0x00 => op!(OpMnemonic::CPY, 0, &[0]),
            0x90 => op!(OpMnemonic::BCC, 2, &[2, 3, 4]),
            0xB0 => op!(OpMnemonic::BCS, 2, &[2, 3, 4]),
            0xF0 => op!(OpMnemonic::BEQ, 2, &[2, 3, 4]),
            // 0x00 => op!(OpMnemonic::BNE, 0, &[0]),
            // 0x00 => op!(OpMnemonic::BPL, 0, &[0]),
            // 0x00 => op!(OpMnemonic::BMI, 0, &[0]),
            // 0x00 => op!(OpMnemonic::BVC, 0, &[0]),
            // 0x00 => op!(OpMnemonic::BVS, 0, &[0]),
            0x4C => op!(OpMnemonic::JMP, 3, &[3]),
            0x20 => op!(OpMnemonic::JSR, 3, &[6]),
            // 0x00 => op!(OpMnemonic::RTS, 0, &[0]),
            // 0x00 => op!(OpMnemonic::BRK, 0, &[0]),
            // 0x00 => op!(OpMnemonic::RTI, 0, &[0]),
            // 0x00 => op!(OpMnemonic::PHA, 0, &[0]),
            // 0x00 => op!(OpMnemonic::PLA, 0, &[0]),
            // 0x00 => op!(OpMnemonic::PHP, 0, &[0]),
            // 0x00 => op!(OpMnemonic::PLP, 0, &[0]),
            // 0x00 => op!(OpMnemonic::TXS, 0, &[0]),
            // 0x00 => op!(OpMnemonic::TSX, 0, &[0]),
            0x18 => op!(OpMnemonic::CLC, 1, &[2]),
            0x38 => op!(OpMnemonic::SEC, 1, &[2]),
            // 0x00 => op!(OpMnemonic::CLI, 0, &[0]),
            // 0x00 => op!(OpMnemonic::SEI, 0, &[0]),
            // 0x00 => op!(OpMnemonic::CLD, 0, &[0]),
            // 0x00 => op!(OpMnemonic::SED, 0, &[0]),
            // 0x00 => op!(OpMnemonic::CLV, 0, &[0]),
            0xEA => op!(OpMnemonic::NOP, 1, &[2]),
            _ => None,
        }
    }
}

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

fn form_u16(bytes: &[u8]) -> u16 {
    ((bytes[2] as u16) << 8) | bytes[1] as u16
}

#[derive(Clone)]
pub enum AddressingMode {
    Immediate,
    ZeroPage,
    ZeroPageX,
    Absolute,
    AbsoluteX,
    AbsoluteY,
    IndirectX,
    IndirectY,
}

#[derive(Clone)]
pub struct Cpu {
    pub sp: usize,
    pub pc: u16,
    pub p: Flags,
    pub a: u8,
    pub x: u8,
    pub y: u8,
    pub mode: AddressingMode,
    pub cycle_count: usize,
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
            mode: AddressingMode::Immediate,
            cycle_count: 0,
        }
    }

    fn fetch(&self, bus: &Bus) -> u8 {
        let opcode = bus.read_byte(self.pc as usize);
        opcode
    }

    fn decode(&self, opcode: u8) -> Option<Op> {
        let op = Op::from_opcode(opcode);
        op
    }

    fn execute(&mut self, opcode: u8, op: Op, bus: &mut Bus) {
        let full_op = bus.read(self.pc, op.bytes);
        let mut cycles = op.cycles[0];

        debug!(
            "{:04X}  {:9} {} ${:26} A:{:02X} X:{:02X} Y:{:02X} P:{:02X} SP:{:02X} PPU:  0, 21 CYC:{}",
            self.pc,
            full_op
                .iter()
                .map(|c| format!("{:02X}", c))
                .collect::<Vec<String>>()
                .join(" "),
            op.mnemonic,
            match op.bytes {
                3 => format!("{:X}{:X}", full_op[2], full_op[1]),
                _ => "".to_string(),
            },
            self.a,
            self.x,
            self.y,
            self.p.bits(),
            self.sp,
            self.cycle_count
        );

        self.pc += op.bytes;
        match op.mnemonic {
            OpMnemonic::LDA => {
                let imm = full_op[1];
                self.x = imm;
                self.p.set(Flags::Z, self.x == 0);
                self.p.set(Flags::N, (self.x >> 7) & 1 == 1);
            }
            // OpMnemonic::STA => {},
            OpMnemonic::LDX => {
                let imm = full_op[1];
                self.x = imm;
                self.p.set(Flags::Z, self.x == 0);
                self.p.set(Flags::N, (self.x >> 7) & 1 == 1);
            }
            OpMnemonic::STX => {
                let low_byte = full_op[1];
                let addr = low_byte as usize;
                bus.write_byte(addr, self.x);
            }
            // OpMnemonic::LDY => {},
            // OpMnemonic::STY => {},
            // OpMnemonic::TAX => {},
            // OpMnemonic::TXA => {},
            // OpMnemonic::TAY => {},
            // OpMnemonic::TYA => {},
            // OpMnemonic::ADC => {},
            // OpMnemonic::SBC => {},
            // OpMnemonic::INC => {},
            // OpMnemonic::DEC => {},
            // OpMnemonic::INX => {},
            // OpMnemonic::DEX => {},
            // OpMnemonic::INY => {},
            // OpMnemonic::DEY => {},
            // OpMnemonic::ASL => {},
            // OpMnemonic::LSR => {},
            // OpMnemonic::ROL => {},
            // OpMnemonic::ROR => {},
            // OpMnemonic::AND => {},
            // OpMnemonic::ORA => {},
            // OpMnemonic::EOR => {},
            // OpMnemonic::BIT => {},
            // OpMnemonic::CMP => {},
            // OpMnemonic::CPX => {},
            // OpMnemonic::CPY => {},
            OpMnemonic::BCC => {
                if !self.p.contains(Flags::C) {
                    let offset = full_op[1] as i16;
                    let new_pc = self.pc.checked_add_signed(offset).unwrap() as u16;
                    self.pc = new_pc;
                    cycles = op.cycles[1];
                }
            }
            OpMnemonic::BCS => {
                if self.p.contains(Flags::C) {
                    let offset = full_op[1] as i16;
                    let new_pc = self.pc.checked_add_signed(offset).unwrap() as u16;
                    self.pc = new_pc;
                    cycles = op.cycles[1];
                }
            }
            // OpMnemonic::BEQ => {},
            // OpMnemonic::BNE => {},
            // OpMnemonic::BPL => {},
            // OpMnemonic::BMI => {},
            // OpMnemonic::BVC => {},
            // OpMnemonic::BVS => {},
            //
            OpMnemonic::JMP => {
                let pc = form_u16(full_op);
                self.pc = pc;
            }
            OpMnemonic::JSR => {
                let addr = form_u16(full_op);
                let stack_i = self.sp + 0x100;
                if stack_i == 0 {
                    error!("Stack overflow");
                    return;
                }
                bus.write_byte(stack_i, (self.pc >> 8) as u8 & 0xff);
                bus.write_byte(stack_i - 1, (self.pc & 0xff) as u8);
                self.sp -= 2;
                self.pc = addr;
            }
            // OpMnemonic::RTS => {},
            // OpMnemonic::BRK => {},
            // OpMnemonic::RTI => {},
            // OpMnemonic::PHA => {},
            // OpMnemonic::PLA => {},
            // OpMnemonic::PHP => {},
            // OpMnemonic::PLP => {},
            // OpMnemonic::TXS => {},
            // OpMnemonic::TSX => {},
            OpMnemonic::CLC => {
                self.p.set(Flags::C, false);
            }
            OpMnemonic::SEC => {
                self.p.set(Flags::C, true);
            }
            // OpMnemonic::CLI => {},
            // OpMnemonic::SEI => {},
            // OpMnemonic::CLD => {},
            // OpMnemonic::SED => {},
            // OpMnemonic::CLV => {},
            OpMnemonic::NOP => {}
            _ => {
                warn!("Not implemented opcode 0x{:02X}", opcode);
            }
        };
        self.cycle_count += cycles;
    }

    pub fn step(&mut self, bus: &mut Bus) {
        let opcode = self.fetch(bus);
        let op = self.decode(opcode);

        match op {
            Some(op) => self.execute(opcode, op, bus),
            None => warn!("Not found opcode 0x{:02X}", opcode),
        }
    }
}
