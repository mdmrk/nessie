use core::fmt;

use bitflags::bitflags;
use log::{debug, warn};

use crate::bus::Bus;

macro_rules! op {
    ($mnemonic:expr, $bytes:expr, $cycles:expr) => {
        Some(Op {
            mnemonic: $mnemonic,
            bytes: $bytes,
            cycles: $cycles,
        })
    };
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
}

impl fmt::Display for OpMnemonic {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub struct Op {
    pub mnemonic: OpMnemonic,
    pub bytes: u16,
    pub cycles: usize,
}

impl Op {
    pub fn from_opcode(opcode: u8) -> Option<Self> {
        match opcode {
            // 0x00 => op!(OpMnemonic::LDA, 0, 0),
            // 0x00 => op!(OpMnemonic::STA, 0, 0),
            0xA2 => op!(OpMnemonic::LDX, 2, 2),
            0x86 => op!(OpMnemonic::STX, 2, 3),
            // 0x00 => op!(OpMnemonic::LDY, 0, 0),
            // 0x00 => op!(OpMnemonic::STY, 0, 0),
            // 0x00 => op!(OpMnemonic::TAX, 0, 0),
            // 0x00 => op!(OpMnemonic::TXA, 0, 0),
            // 0x00 => op!(OpMnemonic::TAY, 0, 0),
            // 0x00 => op!(OpMnemonic::TYA, 0, 0),
            // 0x00 => op!(OpMnemonic::ADC, 0, 0),
            // 0x00 => op!(OpMnemonic::SBC, 0, 0),
            // 0x00 => op!(OpMnemonic::INC, 0, 0),
            // 0x00 => op!(OpMnemonic::DEC, 0, 0),
            // 0x00 => op!(OpMnemonic::INX, 0, 0),
            // 0x00 => op!(OpMnemonic::DEX, 0, 0),
            // 0x00 => op!(OpMnemonic::INY, 0, 0),
            // 0x00 => op!(OpMnemonic::DEY, 0, 0),
            // 0x00 => op!(OpMnemonic::ASL, 0, 0),
            // 0x00 => op!(OpMnemonic::LSR, 0, 0),
            // 0x00 => op!(OpMnemonic::ROL, 0, 0),
            // 0x00 => op!(OpMnemonic::ROR, 0, 0),
            // 0x00 => op!(OpMnemonic::AND, 0, 0),
            // 0x00 => op!(OpMnemonic::ORA, 0, 0),
            // 0x00 => op!(OpMnemonic::EOR, 0, 0),
            // 0x00 => op!(OpMnemonic::BIT, 0, 0),
            // 0x00 => op!(OpMnemonic::CMP, 0, 0),
            // 0x00 => op!(OpMnemonic::CPX, 0, 0),
            // 0x00 => op!(OpMnemonic::CPY, 0, 0),
            // 0x00 => op!(OpMnemonic::BCC, 0, 0),
            // 0x00 => op!(OpMnemonic::BCS, 0, 0),
            // 0x00 => op!(OpMnemonic::BEQ, 0, 0),
            // 0x00 => op!(OpMnemonic::BNE, 0, 0),
            // 0x00 => op!(OpMnemonic::BPL, 0, 0),
            // 0x00 => op!(OpMnemonic::BMI, 0, 0),
            // 0x00 => op!(OpMnemonic::BVC, 0, 0),
            // 0x00 => op!(OpMnemonic::BVS, 0, 0),
            0x4C => op!(OpMnemonic::JMP, 3, 3),
            0x20 => op!(OpMnemonic::JSR, 3, 6),
            // 0x00 => op!(OpMnemonic::RTS, 0, 0),
            // 0x00 => op!(OpMnemonic::BRK, 0, 0),
            // 0x00 => op!(OpMnemonic::RTI, 0, 0),
            // 0x00 => op!(OpMnemonic::PHA, 0, 0),
            // 0x00 => op!(OpMnemonic::PLA, 0, 0),
            // 0x00 => op!(OpMnemonic::PHP, 0, 0),
            // 0x00 => op!(OpMnemonic::PLP, 0, 0),
            // 0x00 => op!(OpMnemonic::TXS, 0, 0),
            // 0x00 => op!(OpMnemonic::TSX, 0, 0),
            // 0x00 => op!(OpMnemonic::CLC, 0, 0),
            // 0x00 => op!(OpMnemonic::SEC, 0, 0),
            // 0x00 => op!(OpMnemonic::CLI, 0, 0),
            // 0x00 => op!(OpMnemonic::SEI, 0, 0),
            // 0x00 => op!(OpMnemonic::CLD, 0, 0),
            // 0x00 => op!(OpMnemonic::SED, 0, 0),
            // 0x00 => op!(OpMnemonic::CLV, 0, 0),
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
    pub sp: u8,
    pub pc: u16,
    pub p: Flags,
    pub a: u8,
    pub x: u8,
    pub y: u8,
    pub mode: AddressingMode,
    pub cycle_count: usize,
    pub stack: [u8; 256],
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
            stack: [0; 256],
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
        let all_bytes = bus.read(self.pc, op.bytes);
        debug!(
            "{:04X}  {:9} {} ${:26} A:{:02X} X:{:02X} Y:{:02X} P:{:02X} SP:{:02X} PPU:  0, 21 CYC:{}",
            self.pc,
            all_bytes
                .iter()
                .map(|c| format!("{:02X}", c))
                .collect::<Vec<String>>()
                .join(" "),
            op.mnemonic,
            match op.bytes {
                3 => format!("{:X}{:X}", all_bytes[2], all_bytes[1]),
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
            // OpMnemonic::LDA => {},
            // OpMnemonic::STA => {},
            OpMnemonic::LDX => {
                let imm = all_bytes[1];
                self.x = imm;
                self.p.set(Flags::Z, self.x == 0);
                self.p.set(Flags::N, (self.x >> 7) & 1 == 1);
            }
            OpMnemonic::STX => {
                let low_byte = all_bytes[1];
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
            // OpMnemonic::BCC => {},
            // OpMnemonic::BCS => {},
            // OpMnemonic::BEQ => {},
            // OpMnemonic::BNE => {},
            // OpMnemonic::BPL => {},
            // OpMnemonic::BMI => {},
            // OpMnemonic::BVC => {},
            // OpMnemonic::BVS => {},
            //
            OpMnemonic::JMP => {
                let pc = form_u16(all_bytes);
                self.pc = pc;
            }
            OpMnemonic::JSR => {
                let addr = form_u16(all_bytes);
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
            // OpMnemonic::CLC => {},
            // OpMnemonic::SEC => {},
            // OpMnemonic::CLI => {},
            // OpMnemonic::SEI => {},
            // OpMnemonic::CLD => {},
            // OpMnemonic::SED => {},
            // OpMnemonic::CLV => {},
            _ => {
                warn!("Not implemented opcode 0x{:04X}", opcode);
            }
        };
        self.cycle_count += op.cycles;
    }

    pub fn step(&mut self, bus: &mut Bus) {
        let opcode = self.fetch(bus);
        let op = self.decode(opcode);

        match op {
            Some(op) => self.execute(opcode, op, bus),
            None => warn!("Not found opcode 0x{:04X}", opcode),
        }
    }
}
