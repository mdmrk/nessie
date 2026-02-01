use core::fmt;
use std::collections::VecDeque;

use bitflags::bitflags;
use log::error;
use phf::phf_map;
#[cfg(not(target_arch = "wasm32"))]
use savefile::prelude::*;

use crate::bus::Bus;

const MAX_LOG_SIZE: usize = 3000;

#[derive(Debug)]
pub enum OperandValue {
    Implied,
    Address(u16, bool),
    Value(u8),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AddrMode {
    Implied,
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

impl AddrMode {
    pub fn resolve(&self, cpu: &Cpu, bus: &mut Bus, operands: &[u8]) -> OperandValue {
        match self {
            AddrMode::Implied | AddrMode::Accumulator => OperandValue::Implied,
            AddrMode::Immediate => OperandValue::Value(operands[0]),
            AddrMode::ZeroPage => OperandValue::Address(operands[0] as u16, false),
            AddrMode::ZeroPageX => {
                let addr = operands[0].wrapping_add(cpu.x) as u16;
                OperandValue::Address(addr, false)
            }
            AddrMode::ZeroPageY => {
                let addr = operands[0].wrapping_add(cpu.y) as u16;
                OperandValue::Address(addr, false)
            }
            AddrMode::Absolute => {
                let addr = u16::from_le_bytes([operands[0], operands[1]]);
                OperandValue::Address(addr, false)
            }
            AddrMode::AbsoluteX => {
                let base = u16::from_le_bytes([operands[0], operands[1]]);
                let addr = base.wrapping_add(cpu.x as u16);
                let page_crossed = (base & 0xFF00) != (addr & 0xFF00);
                OperandValue::Address(addr, page_crossed)
            }
            AddrMode::AbsoluteY => {
                let base = u16::from_le_bytes([operands[0], operands[1]]);
                let addr = base.wrapping_add(cpu.y as u16);
                let page_crossed = (base & 0xFF00) != (addr & 0xFF00);
                OperandValue::Address(addr, page_crossed)
            }
            AddrMode::Indirect => {
                let ptr = u16::from_le_bytes([operands[0], operands[1]]);
                let lo = bus.read_byte(ptr);
                let hi = bus.read_byte((ptr & 0xFF00) | ((ptr + 1) & 0x00FF));
                let addr = u16::from_le_bytes([lo, hi]);
                OperandValue::Address(addr, false)
            }
            AddrMode::IndirectX => {
                let ptr = operands[0].wrapping_add(cpu.x);
                let lo = bus.read_byte(ptr as u16);
                let hi = bus.read_byte(ptr.wrapping_add(1) as u16);
                let addr = u16::from_le_bytes([lo, hi]);
                OperandValue::Address(addr, false)
            }
            AddrMode::IndirectY => {
                let ptr = operands[0];
                let lo = bus.read_byte(ptr as u16);
                let hi = bus.read_byte(ptr.wrapping_add(1) as u16);
                let base = u16::from_le_bytes([lo, hi]);
                let addr = base.wrapping_add(cpu.y as u16);
                let page_crossed = (base & 0xFF00) != (addr & 0xFF00);
                OperandValue::Address(addr, page_crossed)
            }
            AddrMode::Relative => {
                let offset = operands[0] as i8;
                let addr = cpu.pc.wrapping_add_signed(offset as i16);
                let page_crossed = (cpu.pc & 0xFF00) != (addr & 0xFF00);
                OperandValue::Address(addr, page_crossed)
            }
        }
    }

    pub fn operand_bytes(&self) -> u16 {
        match self {
            AddrMode::Implied | AddrMode::Accumulator => 0,
            AddrMode::Immediate
            | AddrMode::ZeroPage
            | AddrMode::ZeroPageX
            | AddrMode::ZeroPageY
            | AddrMode::IndirectX
            | AddrMode::IndirectY
            | AddrMode::Relative => 1,
            AddrMode::Absolute | AddrMode::AbsoluteX | AddrMode::AbsoluteY | AddrMode::Indirect => {
                2
            }
        }
    }
}

#[derive(Debug)]
pub enum OpMnemonic {
    LDA,
    STA,
    LDX,
    LAX,
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
    SLO,
    RLA,
    RRA,
    DCP,
    ISC,
    SRE,
    SAX,
}

impl fmt::Display for OpMnemonic {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub struct Op {
    pub mnemonic: OpMnemonic,
    pub mode: AddrMode,
    pub base_cycles: u8,
    pub execute: fn(&mut Cpu, &mut Bus, AddrMode, &[u8]) -> u8,
    pub illegal: bool,
}

macro_rules! op {
    ($mnemonic:expr, $mode:expr, $base_cycles:expr, $execute:expr, $illegal:expr) => {
        Op {
            mnemonic: $mnemonic,
            mode: $mode,
            base_cycles: $base_cycles,
            execute: $execute,
            illegal: $illegal,
        }
    };
}

static OPCODES: phf::Map<u8, Op> = phf_map! {
    0xA9u8 => op!(OpMnemonic::LDA, AddrMode::Immediate  , 2, Cpu::lda, false),
    0xA5u8 => op!(OpMnemonic::LDA, AddrMode::ZeroPage   , 3, Cpu::lda, false),
    0xB5u8 => op!(OpMnemonic::LDA, AddrMode::ZeroPageX  , 4, Cpu::lda, false),
    0xADu8 => op!(OpMnemonic::LDA, AddrMode::Absolute   , 4, Cpu::lda, false),
    0xBDu8 => op!(OpMnemonic::LDA, AddrMode::AbsoluteX  , 4, Cpu::lda, false),
    0xB9u8 => op!(OpMnemonic::LDA, AddrMode::AbsoluteY  , 4, Cpu::lda, false),
    0xA1u8 => op!(OpMnemonic::LDA, AddrMode::IndirectX  , 6, Cpu::lda, false),
    0xB1u8 => op!(OpMnemonic::LDA, AddrMode::IndirectY  , 5, Cpu::lda, false),
    0xA3u8 => op!(OpMnemonic::LAX, AddrMode::IndirectX  , 6, Cpu::lax, true),
    0xA7u8 => op!(OpMnemonic::LAX, AddrMode::ZeroPage   , 3, Cpu::lax, true),
    0xAFu8 => op!(OpMnemonic::LAX, AddrMode::Absolute   , 4, Cpu::lax, true),
    0xB3u8 => op!(OpMnemonic::LAX, AddrMode::IndirectY  , 5, Cpu::lax, true),
    0xB7u8 => op!(OpMnemonic::LAX, AddrMode::ZeroPageY  , 4, Cpu::lax, true),
    0xBFu8 => op!(OpMnemonic::LAX, AddrMode::AbsoluteY  , 4, Cpu::lax, true),
    0x85u8 => op!(OpMnemonic::STA, AddrMode::ZeroPage   , 3, Cpu::sta, false),
    0x95u8 => op!(OpMnemonic::STA, AddrMode::ZeroPageX  , 4, Cpu::sta, false),
    0x8Du8 => op!(OpMnemonic::STA, AddrMode::Absolute   , 4, Cpu::sta, false),
    0x9Du8 => op!(OpMnemonic::STA, AddrMode::AbsoluteX  , 5, Cpu::sta, false),
    0x99u8 => op!(OpMnemonic::STA, AddrMode::AbsoluteY  , 5, Cpu::sta, false),
    0x81u8 => op!(OpMnemonic::STA, AddrMode::IndirectX  , 6, Cpu::sta, false),
    0x91u8 => op!(OpMnemonic::STA, AddrMode::IndirectY  , 6, Cpu::sta, false),
    0xA2u8 => op!(OpMnemonic::LDX, AddrMode::Immediate  , 2, Cpu::ldx, false),
    0xA6u8 => op!(OpMnemonic::LDX, AddrMode::ZeroPage   , 3, Cpu::ldx, false),
    0xB6u8 => op!(OpMnemonic::LDX, AddrMode::ZeroPageY  , 4, Cpu::ldx, false),
    0xAEu8 => op!(OpMnemonic::LDX, AddrMode::Absolute   , 4, Cpu::ldx, false),
    0xBEu8 => op!(OpMnemonic::LDX, AddrMode::AbsoluteY  , 4, Cpu::ldx, false),
    0x86u8 => op!(OpMnemonic::STX, AddrMode::ZeroPage   , 3, Cpu::stx, false),
    0x96u8 => op!(OpMnemonic::STX, AddrMode::ZeroPageY  , 4, Cpu::stx, false),
    0x8Eu8 => op!(OpMnemonic::STX, AddrMode::Absolute   , 4, Cpu::stx, false),
    0xA0u8 => op!(OpMnemonic::LDY, AddrMode::Immediate  , 2, Cpu::ldy, false),
    0xA4u8 => op!(OpMnemonic::LDY, AddrMode::ZeroPage   , 3, Cpu::ldy, false),
    0xB4u8 => op!(OpMnemonic::LDY, AddrMode::ZeroPageX  , 4, Cpu::ldy, false),
    0xACu8 => op!(OpMnemonic::LDY, AddrMode::Absolute   , 4, Cpu::ldy, false),
    0xBCu8 => op!(OpMnemonic::LDY, AddrMode::AbsoluteX  , 4, Cpu::ldy, false),
    0x84u8 => op!(OpMnemonic::STY, AddrMode::ZeroPage   , 3, Cpu::sty, false),
    0x94u8 => op!(OpMnemonic::STY, AddrMode::ZeroPageX  , 4, Cpu::sty, false),
    0x8Cu8 => op!(OpMnemonic::STY, AddrMode::Absolute   , 4, Cpu::sty, false),
    0xAAu8 => op!(OpMnemonic::TAX, AddrMode::Implied    , 2, Cpu::tax, false),
    0x8Au8 => op!(OpMnemonic::TXA, AddrMode::Implied    , 2, Cpu::txa, false),
    0xA8u8 => op!(OpMnemonic::TAY, AddrMode::Implied    , 2, Cpu::tay, false),
    0x98u8 => op!(OpMnemonic::TYA, AddrMode::Implied    , 2, Cpu::tya, false),
    0x69u8 => op!(OpMnemonic::ADC, AddrMode::Immediate  , 2, Cpu::adc, false),
    0x65u8 => op!(OpMnemonic::ADC, AddrMode::ZeroPage   , 3, Cpu::adc, false),
    0x75u8 => op!(OpMnemonic::ADC, AddrMode::ZeroPageX  , 4, Cpu::adc, false),
    0x6Du8 => op!(OpMnemonic::ADC, AddrMode::Absolute   , 4, Cpu::adc, false),
    0x7Du8 => op!(OpMnemonic::ADC, AddrMode::AbsoluteX  , 4, Cpu::adc, false),
    0x79u8 => op!(OpMnemonic::ADC, AddrMode::AbsoluteY  , 4, Cpu::adc, false),
    0x61u8 => op!(OpMnemonic::ADC, AddrMode::IndirectX  , 6, Cpu::adc, false),
    0x71u8 => op!(OpMnemonic::ADC, AddrMode::IndirectY  , 5, Cpu::adc, false),
    0xE9u8 |
    0xEBu8 => op!(OpMnemonic::SBC, AddrMode::Immediate  , 2, Cpu::sbc, false),
    0xE5u8 => op!(OpMnemonic::SBC, AddrMode::ZeroPage   , 3, Cpu::sbc, false),
    0xF5u8 => op!(OpMnemonic::SBC, AddrMode::ZeroPageX  , 4, Cpu::sbc, false),
    0xEDu8 => op!(OpMnemonic::SBC, AddrMode::Absolute   , 4, Cpu::sbc, false),
    0xFDu8 => op!(OpMnemonic::SBC, AddrMode::AbsoluteX  , 4, Cpu::sbc, false),
    0xF9u8 => op!(OpMnemonic::SBC, AddrMode::AbsoluteY  , 4, Cpu::sbc, false),
    0xE1u8 => op!(OpMnemonic::SBC, AddrMode::IndirectX  , 6, Cpu::sbc, false),
    0xF1u8 => op!(OpMnemonic::SBC, AddrMode::IndirectY  , 5, Cpu::sbc, false),
    0xE6u8 => op!(OpMnemonic::INC, AddrMode::ZeroPage   , 5, Cpu::inc, false),
    0xF6u8 => op!(OpMnemonic::INC, AddrMode::ZeroPageX  , 6, Cpu::inc, false),
    0xEEu8 => op!(OpMnemonic::INC, AddrMode::Absolute   , 6, Cpu::inc, false),
    0xFEu8 => op!(OpMnemonic::INC, AddrMode::AbsoluteX  , 7, Cpu::inc, false),
    0xC6u8 => op!(OpMnemonic::DEC, AddrMode::ZeroPage   , 5, Cpu::dec, false),
    0xD6u8 => op!(OpMnemonic::DEC, AddrMode::ZeroPageX  , 6, Cpu::dec, false),
    0xCEu8 => op!(OpMnemonic::DEC, AddrMode::Absolute   , 6, Cpu::dec, false),
    0xDEu8 => op!(OpMnemonic::DEC, AddrMode::AbsoluteX  , 7, Cpu::dec, false),
    0xE8u8 => op!(OpMnemonic::INX, AddrMode::Implied    , 2, Cpu::inx, false),
    0xCAu8 => op!(OpMnemonic::DEX, AddrMode::Implied    , 2, Cpu::dex, false),
    0xC8u8 => op!(OpMnemonic::INY, AddrMode::Implied    , 2, Cpu::iny, false),
    0x88u8 => op!(OpMnemonic::DEY, AddrMode::Implied    , 2, Cpu::dey, false),
    0x0Au8 => op!(OpMnemonic::ASL, AddrMode::Accumulator, 2, Cpu::asl, false),
    0x06u8 => op!(OpMnemonic::ASL, AddrMode::ZeroPage   , 5, Cpu::asl, false),
    0x16u8 => op!(OpMnemonic::ASL, AddrMode::ZeroPageX  , 6, Cpu::asl, false),
    0x0Eu8 => op!(OpMnemonic::ASL, AddrMode::Absolute   , 6, Cpu::asl, false),
    0x1Eu8 => op!(OpMnemonic::ASL, AddrMode::AbsoluteX  , 7, Cpu::asl, false),
    0x4Au8 => op!(OpMnemonic::LSR, AddrMode::Accumulator, 2, Cpu::lsr, false),
    0x46u8 => op!(OpMnemonic::LSR, AddrMode::ZeroPage   , 5, Cpu::lsr, false),
    0x56u8 => op!(OpMnemonic::LSR, AddrMode::ZeroPageX  , 6, Cpu::lsr, false),
    0x4Eu8 => op!(OpMnemonic::LSR, AddrMode::Absolute   , 6, Cpu::lsr, false),
    0x5Eu8 => op!(OpMnemonic::LSR, AddrMode::AbsoluteX  , 7, Cpu::lsr, false),
    0x2Au8 => op!(OpMnemonic::ROL, AddrMode::Accumulator, 2, Cpu::rol, false),
    0x26u8 => op!(OpMnemonic::ROL, AddrMode::ZeroPage   , 5, Cpu::rol, false),
    0x36u8 => op!(OpMnemonic::ROL, AddrMode::ZeroPageX  , 6, Cpu::rol, false),
    0x2Eu8 => op!(OpMnemonic::ROL, AddrMode::Absolute   , 6, Cpu::rol, false),
    0x3Eu8 => op!(OpMnemonic::ROL, AddrMode::AbsoluteX  , 7, Cpu::rol, false),
    0x6Au8 => op!(OpMnemonic::ROR, AddrMode::Accumulator, 2, Cpu::ror, false),
    0x66u8 => op!(OpMnemonic::ROR, AddrMode::ZeroPage   , 5, Cpu::ror, false),
    0x76u8 => op!(OpMnemonic::ROR, AddrMode::ZeroPageX  , 6, Cpu::ror, false),
    0x6Eu8 => op!(OpMnemonic::ROR, AddrMode::Absolute   , 6, Cpu::ror, false),
    0x7Eu8 => op!(OpMnemonic::ROR, AddrMode::AbsoluteX  , 7, Cpu::ror, false),
    0x29u8 => op!(OpMnemonic::AND, AddrMode::Immediate  , 2, Cpu::and, false),
    0x25u8 => op!(OpMnemonic::AND, AddrMode::ZeroPage   , 3, Cpu::and, false),
    0x35u8 => op!(OpMnemonic::AND, AddrMode::ZeroPageX  , 4, Cpu::and, false),
    0x2Du8 => op!(OpMnemonic::AND, AddrMode::Absolute   , 4, Cpu::and, false),
    0x3Du8 => op!(OpMnemonic::AND, AddrMode::AbsoluteX  , 4, Cpu::and, false),
    0x39u8 => op!(OpMnemonic::AND, AddrMode::AbsoluteY  , 4, Cpu::and, false),
    0x21u8 => op!(OpMnemonic::AND, AddrMode::IndirectX  , 6, Cpu::and, false),
    0x31u8 => op!(OpMnemonic::AND, AddrMode::IndirectY  , 5, Cpu::and, false),
    0x09u8 => op!(OpMnemonic::ORA, AddrMode::Immediate  , 2, Cpu::ora, false),
    0x05u8 => op!(OpMnemonic::ORA, AddrMode::ZeroPage   , 3, Cpu::ora, false),
    0x15u8 => op!(OpMnemonic::ORA, AddrMode::ZeroPageX  , 4, Cpu::ora, false),
    0x0Du8 => op!(OpMnemonic::ORA, AddrMode::Absolute   , 4, Cpu::ora, false),
    0x1Du8 => op!(OpMnemonic::ORA, AddrMode::AbsoluteX  , 4, Cpu::ora, false),
    0x19u8 => op!(OpMnemonic::ORA, AddrMode::AbsoluteY  , 4, Cpu::ora, false),
    0x01u8 => op!(OpMnemonic::ORA, AddrMode::IndirectX  , 6, Cpu::ora, false),
    0x11u8 => op!(OpMnemonic::ORA, AddrMode::IndirectY  , 5, Cpu::ora, false),
    0x49u8 => op!(OpMnemonic::EOR, AddrMode::Immediate  , 2, Cpu::eor, false),
    0x45u8 => op!(OpMnemonic::EOR, AddrMode::ZeroPage   , 3, Cpu::eor, false),
    0x55u8 => op!(OpMnemonic::EOR, AddrMode::ZeroPageX  , 4, Cpu::eor, false),
    0x4Du8 => op!(OpMnemonic::EOR, AddrMode::Absolute   , 4, Cpu::eor, false),
    0x5Du8 => op!(OpMnemonic::EOR, AddrMode::AbsoluteX  , 4, Cpu::eor, false),
    0x59u8 => op!(OpMnemonic::EOR, AddrMode::AbsoluteY  , 4, Cpu::eor, false),
    0x41u8 => op!(OpMnemonic::EOR, AddrMode::IndirectX  , 6, Cpu::eor, false),
    0x51u8 => op!(OpMnemonic::EOR, AddrMode::IndirectY  , 5, Cpu::eor, false),
    0x24u8 => op!(OpMnemonic::BIT, AddrMode::ZeroPage   , 3, Cpu::bit, false),
    0x2Cu8 => op!(OpMnemonic::BIT, AddrMode::Absolute   , 4, Cpu::bit, false),
    0xC9u8 => op!(OpMnemonic::CMP, AddrMode::Immediate  , 2, Cpu::cmp, false),
    0xC5u8 => op!(OpMnemonic::CMP, AddrMode::ZeroPage   , 3, Cpu::cmp, false),
    0xD5u8 => op!(OpMnemonic::CMP, AddrMode::ZeroPageX  , 4, Cpu::cmp, false),
    0xCDu8 => op!(OpMnemonic::CMP, AddrMode::Absolute   , 4, Cpu::cmp, false),
    0xDDu8 => op!(OpMnemonic::CMP, AddrMode::AbsoluteX  , 4, Cpu::cmp, false),
    0xD9u8 => op!(OpMnemonic::CMP, AddrMode::AbsoluteY  , 4, Cpu::cmp, false),
    0xC1u8 => op!(OpMnemonic::CMP, AddrMode::IndirectX  , 6, Cpu::cmp, false),
    0xD1u8 => op!(OpMnemonic::CMP, AddrMode::IndirectY  , 5, Cpu::cmp, false),
    0xE0u8 => op!(OpMnemonic::CPX, AddrMode::Immediate  , 2, Cpu::cpx, false),
    0xE4u8 => op!(OpMnemonic::CPX, AddrMode::ZeroPage   , 3, Cpu::cpx, false),
    0xECu8 => op!(OpMnemonic::CPX, AddrMode::Absolute   , 4, Cpu::cpx, false),
    0xC0u8 => op!(OpMnemonic::CPY, AddrMode::Immediate  , 2, Cpu::cpy, false),
    0xC4u8 => op!(OpMnemonic::CPY, AddrMode::ZeroPage   , 3, Cpu::cpy, false),
    0xCCu8 => op!(OpMnemonic::CPY, AddrMode::Absolute   , 4, Cpu::cpy, false),
    0x90u8 => op!(OpMnemonic::BCC, AddrMode::Relative   , 2, Cpu::bcc, false),
    0xB0u8 => op!(OpMnemonic::BCS, AddrMode::Relative   , 2, Cpu::bcs, false),
    0xF0u8 => op!(OpMnemonic::BEQ, AddrMode::Relative   , 2, Cpu::beq, false),
    0xD0u8 => op!(OpMnemonic::BNE, AddrMode::Relative   , 2, Cpu::bne, false),
    0x10u8 => op!(OpMnemonic::BPL, AddrMode::Relative   , 2, Cpu::bpl, false),
    0x30u8 => op!(OpMnemonic::BMI, AddrMode::Relative   , 2, Cpu::bmi, false),
    0x50u8 => op!(OpMnemonic::BVC, AddrMode::Relative   , 2, Cpu::bvc, false),
    0x70u8 => op!(OpMnemonic::BVS, AddrMode::Relative   , 2, Cpu::bvs, false),
    0x4Cu8 => op!(OpMnemonic::JMP, AddrMode::Absolute   , 3, Cpu::jmp, false),
    0x6Cu8 => op!(OpMnemonic::JMP, AddrMode::Indirect   , 5, Cpu::jmp, false),
    0x20u8 => op!(OpMnemonic::JSR, AddrMode::Absolute   , 6, Cpu::jsr, false),
    0x60u8 => op!(OpMnemonic::RTS, AddrMode::Implied    , 6, Cpu::rts, false),
    0x00u8 => op!(OpMnemonic::BRK, AddrMode::Immediate  , 7, Cpu::brk, false),
    0x40u8 => op!(OpMnemonic::RTI, AddrMode::Implied    , 6, Cpu::rti, false),
    0x48u8 => op!(OpMnemonic::PHA, AddrMode::Implied    , 3, Cpu::pha, false),
    0x68u8 => op!(OpMnemonic::PLA, AddrMode::Implied    , 4, Cpu::pla, false),
    0x08u8 => op!(OpMnemonic::PHP, AddrMode::Implied    , 3, Cpu::php, false),
    0x28u8 => op!(OpMnemonic::PLP, AddrMode::Implied    , 4, Cpu::plp, false),
    0x9Au8 => op!(OpMnemonic::TXS, AddrMode::Implied    , 2, Cpu::txs, false),
    0xBAu8 => op!(OpMnemonic::TSX, AddrMode::Implied    , 2, Cpu::tsx, false),
    0x18u8 => op!(OpMnemonic::CLC, AddrMode::Implied    , 2, Cpu::clc, false),
    0x38u8 => op!(OpMnemonic::SEC, AddrMode::Implied    , 2, Cpu::sec, false),
    0x58u8 => op!(OpMnemonic::CLI, AddrMode::Implied    , 2, Cpu::cli, false),
    0x78u8 => op!(OpMnemonic::SEI, AddrMode::Implied    , 2, Cpu::sei, false),
    0xD8u8 => op!(OpMnemonic::CLD, AddrMode::Implied    , 2, Cpu::cld, false),
    0xF8u8 => op!(OpMnemonic::SED, AddrMode::Implied    , 2, Cpu::sed, false),
    0xB8u8 => op!(OpMnemonic::CLV, AddrMode::Implied    , 2, Cpu::clv, false),
    0xEAu8 => op!(OpMnemonic::NOP, AddrMode::Implied    , 2, Cpu::nop, false),
    0x04u8 |
    0x44u8 |
    0x64u8 => op!(OpMnemonic::NOP, AddrMode::ZeroPage   , 3, Cpu::inop, true),
    0x0Cu8 => op!(OpMnemonic::NOP, AddrMode::Absolute   , 4, Cpu::inop, true),
    0x14u8 |
    0x34u8 |
    0x54u8 |
    0x74u8 |
    0xD4u8 |
    0xF4u8 => op!(OpMnemonic::NOP, AddrMode::ZeroPageX  , 4, Cpu::inop, true),
    0x1Au8 |
    0x3Au8 |
    0x5Au8 |
    0x7Au8 |
    0xDAu8 |
    0xFAu8 => op!(OpMnemonic::NOP, AddrMode::Implied    , 2, Cpu::inop, true),
    0x80u8 |
    0x82u8 |
    0x89u8 |
    0xC2u8 |
    0xE2u8 => op!(OpMnemonic::NOP, AddrMode::Immediate  , 2, Cpu::inop, true),
    0x1Cu8 |
    0x3Cu8 |
    0x5Cu8 |
    0x7Cu8 |
    0xDCu8 |
    0xFCu8 => op!(OpMnemonic::NOP, AddrMode::AbsoluteX  , 4, Cpu::inop, true),
    0x03u8 => op!(OpMnemonic::SLO, AddrMode::IndirectX  , 8, Cpu::slo,  true),
    0x07u8 => op!(OpMnemonic::SLO, AddrMode::ZeroPage   , 5, Cpu::slo,  true),
    0x0Fu8 => op!(OpMnemonic::SLO, AddrMode::Absolute   , 6, Cpu::slo,  true),
    0x13u8 => op!(OpMnemonic::SLO, AddrMode::IndirectY  , 8, Cpu::slo,  true),
    0x17u8 => op!(OpMnemonic::SLO, AddrMode::ZeroPageX  , 6, Cpu::slo,  true),
    0x1Bu8 => op!(OpMnemonic::SLO, AddrMode::AbsoluteY  , 7, Cpu::slo,  true),
    0x1Fu8 => op!(OpMnemonic::SLO, AddrMode::AbsoluteX  , 7, Cpu::slo,  true),
    0x23u8 => op!(OpMnemonic::RLA, AddrMode::IndirectX  , 8, Cpu::rla,  true),
    0x27u8 => op!(OpMnemonic::RLA, AddrMode::ZeroPage   , 5, Cpu::rla,  true),
    0x2Fu8 => op!(OpMnemonic::RLA, AddrMode::Absolute   , 6, Cpu::rla,  true),
    0x33u8 => op!(OpMnemonic::RLA, AddrMode::IndirectY  , 8, Cpu::rla,  true),
    0x37u8 => op!(OpMnemonic::RLA, AddrMode::ZeroPageX  , 6, Cpu::rla,  true),
    0x3Bu8 => op!(OpMnemonic::RLA, AddrMode::AbsoluteY  , 7, Cpu::rla,  true),
    0x3Fu8 => op!(OpMnemonic::RLA, AddrMode::AbsoluteX  , 7, Cpu::rla,  true),
    0x63u8 => op!(OpMnemonic::RRA, AddrMode::IndirectX  , 8, Cpu::rra,  true),
    0x67u8 => op!(OpMnemonic::RRA, AddrMode::ZeroPage   , 5, Cpu::rra,  true),
    0x6Fu8 => op!(OpMnemonic::RRA, AddrMode::Absolute   , 6, Cpu::rra,  true),
    0x73u8 => op!(OpMnemonic::RRA, AddrMode::IndirectY  , 8, Cpu::rra,  true),
    0x77u8 => op!(OpMnemonic::RRA, AddrMode::ZeroPageX  , 6, Cpu::rra,  true),
    0x7Bu8 => op!(OpMnemonic::RRA, AddrMode::AbsoluteY  , 7, Cpu::rra,  true),
    0x7Fu8 => op!(OpMnemonic::RRA, AddrMode::AbsoluteX  , 7, Cpu::rra,  true),
    0xC3u8 => op!(OpMnemonic::DCP, AddrMode::IndirectX  , 8, Cpu::dcp,  true),
    0xC7u8 => op!(OpMnemonic::DCP, AddrMode::ZeroPage   , 5, Cpu::dcp,  true),
    0xCFu8 => op!(OpMnemonic::DCP, AddrMode::Absolute   , 6, Cpu::dcp,  true),
    0xD3u8 => op!(OpMnemonic::DCP, AddrMode::IndirectY  , 8, Cpu::dcp,  true),
    0xD7u8 => op!(OpMnemonic::DCP, AddrMode::ZeroPageX  , 6, Cpu::dcp,  true),
    0xDBu8 => op!(OpMnemonic::DCP, AddrMode::AbsoluteY  , 7, Cpu::dcp,  true),
    0xDFu8 => op!(OpMnemonic::DCP, AddrMode::AbsoluteX  , 7, Cpu::dcp,  true),
    0xE3u8 => op!(OpMnemonic::ISC, AddrMode::IndirectX  , 8, Cpu::isc,  true),
    0xE7u8 => op!(OpMnemonic::ISC, AddrMode::ZeroPage   , 5, Cpu::isc,  true),
    0xEFu8 => op!(OpMnemonic::ISC, AddrMode::Absolute   , 6, Cpu::isc,  true),
    0xF3u8 => op!(OpMnemonic::ISC, AddrMode::IndirectY  , 8, Cpu::isc,  true),
    0xF7u8 => op!(OpMnemonic::ISC, AddrMode::ZeroPageX  , 6, Cpu::isc,  true),
    0xFBu8 => op!(OpMnemonic::ISC, AddrMode::AbsoluteY  , 7, Cpu::isc,  true),
    0xFFu8 => op!(OpMnemonic::ISC, AddrMode::AbsoluteX  , 7, Cpu::isc,  true),
    0x43u8 => op!(OpMnemonic::SRE, AddrMode::IndirectX  , 8, Cpu::sre,  true),
    0x47u8 => op!(OpMnemonic::SRE, AddrMode::ZeroPage   , 5, Cpu::sre,  true),
    0x4Fu8 => op!(OpMnemonic::SRE, AddrMode::Absolute   , 6, Cpu::sre,  true),
    0x53u8 => op!(OpMnemonic::SRE, AddrMode::IndirectY  , 8, Cpu::sre,  true),
    0x57u8 => op!(OpMnemonic::SRE, AddrMode::ZeroPageX  , 6, Cpu::sre,  true),
    0x5Bu8 => op!(OpMnemonic::SRE, AddrMode::AbsoluteY  , 7, Cpu::sre,  true),
    0x5Fu8 => op!(OpMnemonic::SRE, AddrMode::AbsoluteX  , 7, Cpu::sre,  true),
    0x83u8 => op!(OpMnemonic::SAX, AddrMode::IndirectX  , 8, Cpu::sax,  true),
    0x87u8 => op!(OpMnemonic::SAX, AddrMode::ZeroPage   , 5, Cpu::sax,  true),
    0x8Fu8 => op!(OpMnemonic::SAX, AddrMode::Absolute   , 6, Cpu::sax,  true),
    0x97u8 => op!(OpMnemonic::SAX, AddrMode::ZeroPageY  , 8, Cpu::sax,  true),
};

bitflags! {
    #[derive(Debug, Clone, Copy)]
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

#[cfg(not(target_arch = "wasm32"))]
impl WithSchema for Flags {
    fn schema(_version: u32, _context: &mut WithSchemaContext) -> Schema {
        Schema::Primitive(SchemaPrimitive::schema_u8)
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Serialize for Flags {
    fn serialize(
        &self,
        serializer: &mut Serializer<impl std::io::Write>,
    ) -> Result<(), SavefileError> {
        self.bits().serialize(serializer)
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Deserialize for Flags {
    fn deserialize(
        deserializer: &mut Deserializer<impl std::io::Read>,
    ) -> Result<Self, SavefileError> {
        let raw = u8::deserialize(deserializer)?;
        Ok(Flags::from_bits_retain(raw))
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Packed for Flags {}

#[derive(Clone)]
#[cfg_attr(not(target_arch = "wasm32"), derive(Savefile))]
pub struct Cpu {
    pub sp: u8,
    pub pc: u16,
    #[cfg_attr(not(target_arch = "wasm32"), savefile_introspect_ignore)]
    pub p: Flags,
    pub a: u8,
    pub x: u8,
    pub y: u8,
    pub cycles: u64,
    pub nmi_pending: bool,
    pub nmi_previous_state: bool,
    pub irq_pending: bool,
    #[cfg_attr(not(target_arch = "wasm32"), savefile_introspect_ignore)]
    #[cfg_attr(not(target_arch = "wasm32"), savefile_ignore)]
    pub log: Option<VecDeque<char>>,
}

impl Default for Cpu {
    fn default() -> Self {
        Self::new(false)
    }
}

impl Cpu {
    pub fn new(enable_logging: bool) -> Self {
        Self {
            sp: 0xfd,
            pc: 0,
            p: Flags::I | Flags::_1,
            a: 0,
            x: 0,
            y: 0,
            cycles: 7,
            nmi_pending: false,
            nmi_previous_state: false,
            irq_pending: false,
            log: if enable_logging {
                Some(VecDeque::with_capacity(MAX_LOG_SIZE))
            } else {
                None
            },
        }
    }

    pub fn reset(&mut self, bus: &mut Bus) {
        let lo = bus.read_byte(0xFFFC);
        let hi = bus.read_byte(0xFFFD);
        self.pc = u16::from_le_bytes([lo, hi]);

        self.sp = 0xFD;
        self.p = Flags::I | Flags::_1;
        self.cycles = 7;
    }

    fn fetch(&self, bus: &mut Bus) -> u8 {
        bus.read_byte(self.pc)
    }

    fn decode(&self, opcode: u8) -> Option<&'static Op> {
        OPCODES.get(&opcode)
    }

    fn execute(&mut self, bus: &mut Bus, op: &Op, opcode: u8) {
        let operand_bytes = op.mode.operand_bytes();
        let operands = bus.read_range(self.pc.wrapping_add(1), operand_bytes);

        if self.log.is_some() {
            self.log(bus, opcode, op, &operands);
        }

        self.pc = self.pc.wrapping_add(1 + operand_bytes);
        let extra_cycles = (op.execute)(self, bus, op.mode, &operands);
        let total_cycles = op.base_cycles + extra_cycles;
        self.cycles += total_cycles as u64;

        bus.ppu
            .step(&mut bus.cart.as_mut().unwrap().mapper, total_cycles);
        for _ in 0..total_cycles {
            bus.apu.step();
        }
        self.irq_pending = bus.apu.irq_occurred();
        let nmi_current_state = bus.ppu.check_nmi();
        if nmi_current_state && !self.nmi_previous_state {
            self.nmi_pending = true;
        }
        self.nmi_previous_state = nmi_current_state;
    }

    fn log(&mut self, bus: &Bus, opcode: u8, op: &Op, operands: &[u8]) {
        let log = self.log.as_mut().unwrap();
        let step_str = format!(
            "{:04X}  {:02X} {:6}{}{} A:{:02X} X:{:02X} Y:{:02X} P:{:02X} SP:{:02X} PPU:{:3},{:3} CYC:{}\n",
            self.pc,
            opcode,
            operands
                .iter()
                .map(|c| format!("{:02X}", c))
                .collect::<Vec<String>>()
                .join(" "),
            if op.illegal { "*" } else { " " },
            op.mnemonic,
            self.a,
            self.x,
            self.y,
            self.p.bits(),
            self.sp,
            bus.ppu.scanline,
            bus.ppu.dot,
            self.cycles
        );
        for c in step_str.chars() {
            log.push_back(c);
        }

        while log.len() > MAX_LOG_SIZE {
            log.pop_front();
        }
    }

    pub fn step(&mut self, bus: &mut Bus) -> Result<(), String> {
        if self.nmi_pending {
            self.handle_nmi(bus);
            self.nmi_pending = false;
            Ok(())
        } else if self.irq_pending && !self.p.contains(Flags::I) {
            self.handle_irq(bus);
            self.irq_pending = false;
            Ok(())
        } else {
            let opcode = self.fetch(bus);
            let op = self.decode(opcode);

            match op {
                Some(op) => {
                    self.execute(bus, op, opcode);
                    Ok(())
                }
                None => {
                    self.pc = self.pc.wrapping_add(1);
                    Err(format!("Unknown opcode: 0x{:02X}", opcode))
                }
            }
        }
    }

    fn handle_nmi(&mut self, bus: &mut Bus) {
        self.push_stack(bus, (self.pc >> 8) as u8);
        self.push_stack(bus, self.pc as u8);

        let mut p = self.p;
        p.remove(Flags::B);
        p.insert(Flags::_1);
        self.push_stack(bus, p.bits());

        self.p.insert(Flags::I);

        let lo = bus.read_byte(0xFFFA);
        let hi = bus.read_byte(0xFFFB);
        self.pc = u16::from_le_bytes([lo, hi]);

        let cycles: u8 = 7;
        self.cycles += cycles as u64;
        bus.ppu.step(&mut bus.cart.as_mut().unwrap().mapper, cycles);
        for _ in 0..cycles {
            bus.apu.step();
        }
    }

    fn handle_irq(&mut self, bus: &mut Bus) {
        self.push_stack(bus, (self.pc >> 8) as u8);
        self.push_stack(bus, self.pc as u8);

        let mut p = self.p;
        p.remove(Flags::B);
        p.insert(Flags::_1);
        self.push_stack(bus, p.bits());

        self.p.insert(Flags::I);

        let lo = bus.read_byte(0xFFFE);
        let hi = bus.read_byte(0xFFFF);
        self.pc = u16::from_le_bytes([lo, hi]);

        let cycles: u8 = 7;
        self.cycles += cycles as u64;
        bus.ppu.step(&mut bus.cart.as_mut().unwrap().mapper, cycles);
        for _ in 0..cycles {
            bus.apu.step();
        }
    }

    fn update_nz(&mut self, value: u8) {
        self.p.set(Flags::Z, value == 0);
        self.p.set(Flags::N, (value >> 7) & 1 == 1);
    }

    fn push_stack(&mut self, bus: &mut Bus, value: u8) {
        bus.write_byte(0x100 + self.sp as u16, value);
        self.sp = self.sp.wrapping_sub(1);
    }

    fn pop_stack(&mut self, bus: &mut Bus) -> u8 {
        self.sp = self.sp.wrapping_add(1);
        bus.read_byte(0x100 + self.sp as u16)
    }

    fn read_operand(&self, bus: &mut Bus, mode: AddrMode, operands: &[u8]) -> (u8, bool) {
        match mode.resolve(self, bus, operands) {
            OperandValue::Value(v) => (v, false),
            OperandValue::Address(addr, crossed) => {
                if crossed {
                    let page = addr & 0xFF00;
                    let dummy_page = page.wrapping_sub(0x100);
                    let dummy_addr = dummy_page | (addr & 0x00FF);
                    bus.read_byte(dummy_addr);
                }
                (bus.read_byte(addr), crossed)
            }
            OperandValue::Implied => (self.a, false),
        }
    }

    fn write_operand(&mut self, bus: &mut Bus, mode: AddrMode, operands: &[u8], value: u8) {
        match mode.resolve(self, bus, operands) {
            OperandValue::Address(addr, crossed) => {
                match mode {
                    AddrMode::AbsoluteX | AddrMode::AbsoluteY | AddrMode::IndirectY => {
                        let dummy_addr = if crossed {
                            (addr & 0x00FF) | (addr.wrapping_sub(0x100) & 0xFF00)
                        } else {
                            addr
                        };
                        bus.read_byte(dummy_addr);
                    }
                    _ => {}
                }
                bus.write_byte(addr, value)
            }
            OperandValue::Implied => self.a = value,
            _ => error!("Cannot write to this addressing mode"),
        }
    }

    fn lda(cpu: &mut Cpu, bus: &mut Bus, mode: AddrMode, operands: &[u8]) -> u8 {
        let (value, page_crossed) = cpu.read_operand(bus, mode, operands);
        cpu.a = value;
        cpu.update_nz(cpu.a);
        if page_crossed { 1 } else { 0 }
    }

    fn lax(cpu: &mut Cpu, bus: &mut Bus, mode: AddrMode, operands: &[u8]) -> u8 {
        let (value, page_crossed) = cpu.read_operand(bus, mode, operands);
        cpu.a = value;
        cpu.update_nz(cpu.a);
        Cpu::tax(cpu, bus, mode, operands);
        if page_crossed { 1 } else { 0 }
    }

    fn sta(cpu: &mut Cpu, bus: &mut Bus, mode: AddrMode, operands: &[u8]) -> u8 {
        cpu.write_operand(bus, mode, operands, cpu.a);
        0
    }

    fn ldx(cpu: &mut Cpu, bus: &mut Bus, mode: AddrMode, operands: &[u8]) -> u8 {
        let (value, page_crossed) = cpu.read_operand(bus, mode, operands);
        cpu.x = value;
        cpu.update_nz(cpu.x);
        if page_crossed { 1 } else { 0 }
    }

    fn stx(cpu: &mut Cpu, bus: &mut Bus, mode: AddrMode, operands: &[u8]) -> u8 {
        cpu.write_operand(bus, mode, operands, cpu.x);
        0
    }

    fn ldy(cpu: &mut Cpu, bus: &mut Bus, mode: AddrMode, operands: &[u8]) -> u8 {
        let (value, page_crossed) = cpu.read_operand(bus, mode, operands);
        cpu.y = value;
        cpu.update_nz(cpu.y);
        if page_crossed { 1 } else { 0 }
    }

    fn sty(cpu: &mut Cpu, bus: &mut Bus, mode: AddrMode, operands: &[u8]) -> u8 {
        cpu.write_operand(bus, mode, operands, cpu.y);
        0
    }

    fn tax(cpu: &mut Cpu, _bus: &mut Bus, _mode: AddrMode, _operands: &[u8]) -> u8 {
        cpu.x = cpu.a;
        cpu.update_nz(cpu.x);
        0
    }

    fn txa(cpu: &mut Cpu, _bus: &mut Bus, _mode: AddrMode, _operands: &[u8]) -> u8 {
        cpu.a = cpu.x;
        cpu.update_nz(cpu.a);
        0
    }

    fn tay(cpu: &mut Cpu, _bus: &mut Bus, _mode: AddrMode, _operands: &[u8]) -> u8 {
        cpu.y = cpu.a;
        cpu.update_nz(cpu.y);
        0
    }

    fn tya(cpu: &mut Cpu, _bus: &mut Bus, _mode: AddrMode, _operands: &[u8]) -> u8 {
        cpu.a = cpu.y;
        cpu.update_nz(cpu.a);
        0
    }

    fn adc(cpu: &mut Cpu, bus: &mut Bus, mode: AddrMode, operands: &[u8]) -> u8 {
        let (value, page_crossed) = cpu.read_operand(bus, mode, operands);
        let carry = if cpu.p.contains(Flags::C) { 1 } else { 0 };
        let old_a = cpu.a;
        let sum = cpu.a as u16 + value as u16 + carry as u16;
        let result = sum as u8;
        cpu.p.set(Flags::C, sum > 0xFF);
        cpu.p
            .set(Flags::V, ((old_a ^ result) & (value ^ result) & 0x80) != 0);
        cpu.a = result;
        cpu.update_nz(cpu.a);
        if page_crossed { 1 } else { 0 }
    }

    fn sbc(cpu: &mut Cpu, bus: &mut Bus, mode: AddrMode, operands: &[u8]) -> u8 {
        let (value, page_crossed) = cpu.read_operand(bus, mode, operands);
        let carry = if cpu.p.contains(Flags::C) { 1 } else { 0 };
        let diff = cpu.a as u16 + (!value as u16) + carry as u16;
        let result = diff as u8;

        cpu.p
            .set(Flags::V, ((cpu.a ^ value) & (cpu.a ^ result) & 0x80) != 0);
        cpu.p.set(Flags::C, diff > 0xFF);
        cpu.a = result;
        cpu.update_nz(cpu.a);
        if page_crossed { 1 } else { 0 }
    }

    fn inc(cpu: &mut Cpu, bus: &mut Bus, mode: AddrMode, operands: &[u8]) -> u8 {
        let (value, _) = cpu.read_operand(bus, mode, operands);
        cpu.write_operand(bus, mode, operands, value);
        let result = value.wrapping_add(1);
        cpu.write_operand(bus, mode, operands, result);
        cpu.update_nz(result);
        0
    }

    fn dec(cpu: &mut Cpu, bus: &mut Bus, mode: AddrMode, operands: &[u8]) -> u8 {
        let (value, _) = cpu.read_operand(bus, mode, operands);
        cpu.write_operand(bus, mode, operands, value);
        let result = value.wrapping_sub(1);
        cpu.write_operand(bus, mode, operands, result);
        cpu.update_nz(result);
        0
    }

    fn inx(cpu: &mut Cpu, _bus: &mut Bus, _mode: AddrMode, _operands: &[u8]) -> u8 {
        cpu.x = cpu.x.wrapping_add(1);
        cpu.update_nz(cpu.x);
        0
    }

    fn dex(cpu: &mut Cpu, _bus: &mut Bus, _mode: AddrMode, _operands: &[u8]) -> u8 {
        cpu.x = cpu.x.wrapping_sub(1);
        cpu.update_nz(cpu.x);
        0
    }

    fn iny(cpu: &mut Cpu, _bus: &mut Bus, _mode: AddrMode, _operands: &[u8]) -> u8 {
        cpu.y = cpu.y.wrapping_add(1);
        cpu.update_nz(cpu.y);
        0
    }

    fn dey(cpu: &mut Cpu, _bus: &mut Bus, _mode: AddrMode, _operands: &[u8]) -> u8 {
        cpu.y = cpu.y.wrapping_sub(1);
        cpu.update_nz(cpu.y);
        0
    }

    fn asl(cpu: &mut Cpu, bus: &mut Bus, mode: AddrMode, operands: &[u8]) -> u8 {
        let (value, _) = cpu.read_operand(bus, mode, operands);
        if mode != AddrMode::Accumulator {
            cpu.write_operand(bus, mode, operands, value);
        }
        let result = value << 1;
        cpu.p.set(Flags::C, (value & 0b1000_0000) != 0);
        cpu.p.set(Flags::Z, result == 0);
        cpu.p.set(Flags::N, (result & 0b1000_0000) != 0);
        cpu.write_operand(bus, mode, operands, result);
        0
    }

    fn lsr(cpu: &mut Cpu, bus: &mut Bus, mode: AddrMode, operands: &[u8]) -> u8 {
        let (value, _) = cpu.read_operand(bus, mode, operands);
        if mode != AddrMode::Accumulator {
            cpu.write_operand(bus, mode, operands, value);
        }
        let result = value >> 1;
        cpu.p.set(Flags::C, (value & 0b1) != 0);
        cpu.p.set(Flags::Z, result == 0);
        cpu.p.set(Flags::N, false);
        cpu.write_operand(bus, mode, operands, result);
        0
    }

    fn rol(cpu: &mut Cpu, bus: &mut Bus, mode: AddrMode, operands: &[u8]) -> u8 {
        let (value, _) = cpu.read_operand(bus, mode, operands);
        if mode != AddrMode::Accumulator {
            cpu.write_operand(bus, mode, operands, value);
        }
        let carry = if cpu.p.contains(Flags::C) { 1 } else { 0 };
        let result = (value << 1) | carry;
        cpu.p.set(Flags::C, ((value >> 7) & 1) != 0);
        cpu.p.set(Flags::Z, result == 0);
        cpu.p.set(Flags::N, (result & 0x80) != 0);
        cpu.write_operand(bus, mode, operands, result);
        0
    }

    fn ror(cpu: &mut Cpu, bus: &mut Bus, mode: AddrMode, operands: &[u8]) -> u8 {
        let (value, _) = cpu.read_operand(bus, mode, operands);
        if mode != AddrMode::Accumulator {
            cpu.write_operand(bus, mode, operands, value);
        }
        let carry = if cpu.p.contains(Flags::C) { 1 } else { 0 };
        let result = (value >> 1) | (carry << 7);
        cpu.p.set(Flags::C, (value & 0b1) != 0);
        cpu.p.set(Flags::Z, result == 0);
        cpu.p.set(Flags::N, (result & 0b1000_0000) != 0);
        cpu.write_operand(bus, mode, operands, result);
        0
    }

    fn and(cpu: &mut Cpu, bus: &mut Bus, mode: AddrMode, operands: &[u8]) -> u8 {
        let (value, page_crossed) = cpu.read_operand(bus, mode, operands);
        cpu.a &= value;
        cpu.update_nz(cpu.a);
        if page_crossed { 1 } else { 0 }
    }

    fn ora(cpu: &mut Cpu, bus: &mut Bus, mode: AddrMode, operands: &[u8]) -> u8 {
        let (value, page_crossed) = cpu.read_operand(bus, mode, operands);
        cpu.a |= value;
        cpu.update_nz(cpu.a);
        if page_crossed { 1 } else { 0 }
    }

    fn eor(cpu: &mut Cpu, bus: &mut Bus, mode: AddrMode, operands: &[u8]) -> u8 {
        let (value, page_crossed) = cpu.read_operand(bus, mode, operands);
        cpu.a ^= value;
        cpu.update_nz(cpu.a);
        if page_crossed { 1 } else { 0 }
    }

    fn bit(cpu: &mut Cpu, bus: &mut Bus, mode: AddrMode, operands: &[u8]) -> u8 {
        let (value, _) = cpu.read_operand(bus, mode, operands);
        let result = value & cpu.a;
        cpu.p.set(Flags::Z, result == 0);
        cpu.p.set(Flags::V, value & 0b0100_0000 != 0);
        cpu.p.set(Flags::N, value & 0b1000_0000 != 0);
        0
    }

    fn cmp(cpu: &mut Cpu, bus: &mut Bus, mode: AddrMode, operands: &[u8]) -> u8 {
        let (value, page_crossed) = cpu.read_operand(bus, mode, operands);
        let result = cpu.a.wrapping_sub(value);
        cpu.p.set(Flags::C, cpu.a >= value);
        cpu.p.set(Flags::Z, cpu.a == value);
        cpu.p.set(Flags::N, result & 0b1000_0000 != 0);
        if page_crossed { 1 } else { 0 }
    }

    fn cpx(cpu: &mut Cpu, bus: &mut Bus, mode: AddrMode, operands: &[u8]) -> u8 {
        let (value, _) = cpu.read_operand(bus, mode, operands);
        let result = cpu.x.wrapping_sub(value);
        cpu.p.set(Flags::C, cpu.x >= value);
        cpu.p.set(Flags::Z, cpu.x == value);
        cpu.p.set(Flags::N, result & 0b1000_0000 != 0);
        0
    }

    fn cpy(cpu: &mut Cpu, bus: &mut Bus, mode: AddrMode, operands: &[u8]) -> u8 {
        let (value, _) = cpu.read_operand(bus, mode, operands);
        let result = cpu.y.wrapping_sub(value);
        cpu.p.set(Flags::C, cpu.y >= value);
        cpu.p.set(Flags::Z, cpu.y == value);
        cpu.p.set(Flags::N, result & 0b1000_0000 != 0);
        0
    }

    fn bcc(cpu: &mut Cpu, bus: &mut Bus, mode: AddrMode, operands: &[u8]) -> u8 {
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

    fn bcs(cpu: &mut Cpu, bus: &mut Bus, mode: AddrMode, operands: &[u8]) -> u8 {
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

    fn beq(cpu: &mut Cpu, bus: &mut Bus, mode: AddrMode, operands: &[u8]) -> u8 {
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

    fn bne(cpu: &mut Cpu, bus: &mut Bus, mode: AddrMode, operands: &[u8]) -> u8 {
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

    fn bpl(cpu: &mut Cpu, bus: &mut Bus, mode: AddrMode, operands: &[u8]) -> u8 {
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

    fn bmi(cpu: &mut Cpu, bus: &mut Bus, mode: AddrMode, operands: &[u8]) -> u8 {
        if cpu.p.contains(Flags::N) {
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

    fn bvc(cpu: &mut Cpu, bus: &mut Bus, mode: AddrMode, operands: &[u8]) -> u8 {
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

    fn bvs(cpu: &mut Cpu, bus: &mut Bus, mode: AddrMode, operands: &[u8]) -> u8 {
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

    fn jmp(cpu: &mut Cpu, bus: &mut Bus, mode: AddrMode, operands: &[u8]) -> u8 {
        if let OperandValue::Address(addr, _) = mode.resolve(cpu, bus, operands) {
            cpu.pc = addr;
        }
        0
    }

    fn jsr(cpu: &mut Cpu, bus: &mut Bus, mode: AddrMode, operands: &[u8]) -> u8 {
        let pc = cpu.pc.wrapping_sub(1);
        cpu.push_stack(bus, (pc >> 8) as u8);
        cpu.push_stack(bus, pc as u8);
        if let OperandValue::Address(addr, _) = mode.resolve(cpu, bus, operands) {
            cpu.pc = addr;
        }
        if operands.len() > 1 {
            bus.open_bus = operands[1];
        }
        0
    }

    fn rts(cpu: &mut Cpu, bus: &mut Bus, _mode: AddrMode, _operands: &[u8]) -> u8 {
        let lo = cpu.pop_stack(bus);
        let hi = cpu.pop_stack(bus);
        cpu.pc = u16::from_le_bytes([lo, hi]).wrapping_add(1);
        0
    }

    fn brk(cpu: &mut Cpu, bus: &mut Bus, _mode: AddrMode, _operands: &[u8]) -> u8 {
        cpu.push_stack(bus, (cpu.pc >> 8) as u8);
        cpu.push_stack(bus, cpu.pc as u8);

        let p = cpu.p | Flags::B | Flags::_1;
        cpu.push_stack(bus, p.bits());

        cpu.p.insert(Flags::I);

        let lo = bus.read_byte(0xFFFE);
        let hi = bus.read_byte(0xFFFF);
        cpu.pc = u16::from_le_bytes([lo, hi]);
        0
    }

    fn rti(cpu: &mut Cpu, bus: &mut Bus, _mode: AddrMode, _operands: &[u8]) -> u8 {
        let mut p = Flags::from_bits(cpu.pop_stack(bus)).unwrap();
        p.remove(Flags::B);
        p.insert(Flags::_1);
        cpu.p = p;
        let lo = cpu.pop_stack(bus);
        let hi = cpu.pop_stack(bus);
        cpu.pc = u16::from_le_bytes([lo, hi]);
        0
    }

    fn pha(cpu: &mut Cpu, bus: &mut Bus, _mode: AddrMode, _operands: &[u8]) -> u8 {
        cpu.push_stack(bus, cpu.a);
        0
    }

    fn pla(cpu: &mut Cpu, bus: &mut Bus, _mode: AddrMode, _operands: &[u8]) -> u8 {
        cpu.a = cpu.pop_stack(bus);
        cpu.update_nz(cpu.a);
        0
    }

    fn php(cpu: &mut Cpu, bus: &mut Bus, _mode: AddrMode, _operands: &[u8]) -> u8 {
        let p = cpu.p | Flags::B | Flags::_1;
        cpu.push_stack(bus, p.bits());
        0
    }

    fn plp(cpu: &mut Cpu, bus: &mut Bus, _mode: AddrMode, _operands: &[u8]) -> u8 {
        let mut p = Flags::from_bits(cpu.pop_stack(bus)).unwrap();
        p.remove(Flags::B);
        p.insert(Flags::_1);
        cpu.p = p;
        0
    }

    fn txs(cpu: &mut Cpu, _bus: &mut Bus, _mode: AddrMode, _operands: &[u8]) -> u8 {
        cpu.sp = cpu.x;
        0
    }

    fn tsx(cpu: &mut Cpu, _bus: &mut Bus, _mode: AddrMode, _operands: &[u8]) -> u8 {
        cpu.x = cpu.sp;
        cpu.update_nz(cpu.x);
        0
    }

    fn clc(cpu: &mut Cpu, _bus: &mut Bus, _mode: AddrMode, _operands: &[u8]) -> u8 {
        cpu.p.set(Flags::C, false);
        0
    }

    fn sec(cpu: &mut Cpu, _bus: &mut Bus, _mode: AddrMode, _operands: &[u8]) -> u8 {
        cpu.p.set(Flags::C, true);
        0
    }

    fn cli(cpu: &mut Cpu, _bus: &mut Bus, _mode: AddrMode, _operands: &[u8]) -> u8 {
        cpu.p.set(Flags::I, false);
        0
    }

    fn sei(cpu: &mut Cpu, _bus: &mut Bus, _mode: AddrMode, _operands: &[u8]) -> u8 {
        cpu.p.set(Flags::I, true);
        0
    }

    fn cld(cpu: &mut Cpu, _bus: &mut Bus, _mode: AddrMode, _operands: &[u8]) -> u8 {
        cpu.p.set(Flags::D, false);
        0
    }

    fn sed(cpu: &mut Cpu, _bus: &mut Bus, _mode: AddrMode, _operands: &[u8]) -> u8 {
        cpu.p.set(Flags::D, true);
        0
    }

    fn clv(cpu: &mut Cpu, _bus: &mut Bus, _mode: AddrMode, _operands: &[u8]) -> u8 {
        cpu.p.set(Flags::V, false);
        0
    }

    fn nop(_cpu: &mut Cpu, _bus: &mut Bus, _mode: AddrMode, _operands: &[u8]) -> u8 {
        0
    }

    fn inop(cpu: &mut Cpu, bus: &mut Bus, mode: AddrMode, operands: &[u8]) -> u8 {
        let (_, page_crossed) = cpu.read_operand(bus, mode, operands);
        if page_crossed { 1 } else { 0 }
    }

    fn slo(cpu: &mut Cpu, bus: &mut Bus, mode: AddrMode, operands: &[u8]) -> u8 {
        Cpu::asl(cpu, bus, mode, operands);
        Cpu::ora(cpu, bus, mode, operands);
        0
    }

    fn rla(cpu: &mut Cpu, bus: &mut Bus, mode: AddrMode, operands: &[u8]) -> u8 {
        Cpu::rol(cpu, bus, mode, operands);
        Cpu::and(cpu, bus, mode, operands);
        0
    }

    fn rra(cpu: &mut Cpu, bus: &mut Bus, mode: AddrMode, operands: &[u8]) -> u8 {
        Cpu::ror(cpu, bus, mode, operands);
        Cpu::adc(cpu, bus, mode, operands);
        0
    }

    fn dcp(cpu: &mut Cpu, bus: &mut Bus, mode: AddrMode, operands: &[u8]) -> u8 {
        Cpu::dec(cpu, bus, mode, operands);
        Cpu::cmp(cpu, bus, mode, operands);
        0
    }

    fn isc(cpu: &mut Cpu, bus: &mut Bus, mode: AddrMode, operands: &[u8]) -> u8 {
        Cpu::inc(cpu, bus, mode, operands);
        Cpu::sbc(cpu, bus, mode, operands);
        0
    }

    fn sre(cpu: &mut Cpu, bus: &mut Bus, mode: AddrMode, operands: &[u8]) -> u8 {
        Cpu::lsr(cpu, bus, mode, operands);
        Cpu::eor(cpu, bus, mode, operands);
        0
    }

    fn sax(cpu: &mut Cpu, bus: &mut Bus, mode: AddrMode, operands: &[u8]) -> u8 {
        cpu.write_operand(bus, mode, operands, cpu.x & cpu.a);
        0
    }
}
