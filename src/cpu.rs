use core::fmt;

use bitflags::bitflags;
use log::warn;

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
            0x4C => op!(OpMnemonic::JMP, 3, 3),
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

    fn execute(&mut self, opcode: u8, op: Op, bus: &Bus) {
        let all_bytes = bus.read(self.pc, op.bytes);
        println!(
            "{:04X}  {:9} {} ${:26} A:{:02} X:{:02} Y:{:02} P:{:02} SP:{:02X} PPU:  0, 21 CYC:{}",
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

        match op.mnemonic {
            OpMnemonic::JMP => {}
            _ => {
                warn!("Not implemented opcode 0x{:04X}", opcode);
            }
        };

        self.pc += op.bytes;
        self.cycle_count += op.cycles;
    }

    pub fn step(&mut self, bus: &Bus) {
        let opcode = self.fetch(bus);
        let op = self.decode(opcode);

        match op {
            Some(op) => self.execute(opcode, op, bus),
            None => warn!("Not found opcode 0x{:04X}", opcode),
        }
    }
}
