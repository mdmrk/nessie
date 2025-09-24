use bitflags::bitflags;
use log::warn;

use crate::bus::Bus;

pub enum OpcodeMnemonic {
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

pub struct Op {
    pub mnemonic: OpcodeMnemonic,
    pub bytes: u8,
    pub cycles: u8,
}

impl Op {
    pub fn from_opcode(opcode: u8) -> Option<Self> {
        match opcode {
            0x4C => Some(Op {
                mnemonic: OpcodeMnemonic::JMP,
                bytes: 3,
                cycles: 3,
            }),
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
    pub flags: Flags,
    pub a: u8,
    pub x: u8,
    pub y: u8,
    pub mode: AddressingMode,
}

impl Cpu {
    pub fn new() -> Self {
        Self {
            sp: 0xfd,
            pc: 0,
            flags: Flags::I | Flags::_1,
            a: 0,
            x: 0,
            y: 0,
            mode: AddressingMode::Immediate,
        }
    }

    fn fetch(&mut self, bus: &Bus) -> u8 {
        let pc = self.pc as usize;
        self.pc += 1;
        return bus.read_byte(pc);
    }

    fn execute(&self, opcode: u8, bus: &Bus) {
        let op = Op::from_opcode(opcode);

        match op {
            Some(op) => match op {
                _ => {
                    warn!("Not implemented opcode 0x{:04X}", opcode);
                }
            },
            None => {
                warn!("Not found opcode 0x{:04X}", opcode);
            }
        }
    }

    pub fn step(&mut self, bus: &Bus) {
        let opcode = self.fetch(bus);
        self.execute(opcode, bus);
    }
}
