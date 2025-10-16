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
    0xA9u8 => op!(OpMnemonic::LDA, AddressingMode::Immediate  , 2, Cpu::lda, false),
    0xA5u8 => op!(OpMnemonic::LDA, AddressingMode::ZeroPage   , 3, Cpu::lda, false),
    0xB5u8 => op!(OpMnemonic::LDA, AddressingMode::ZeroPageX  , 4, Cpu::lda, false),
    0xADu8 => op!(OpMnemonic::LDA, AddressingMode::Absolute   , 4, Cpu::lda, false),
    0xBDu8 => op!(OpMnemonic::LDA, AddressingMode::AbsoluteX  , 4, Cpu::lda, false),
    0xB9u8 => op!(OpMnemonic::LDA, AddressingMode::AbsoluteY  , 4, Cpu::lda, false),
    0xA1u8 => op!(OpMnemonic::LDA, AddressingMode::IndirectX  , 6, Cpu::lda, false),
    0xB1u8 => op!(OpMnemonic::LDA, AddressingMode::IndirectY  , 5, Cpu::lda, false),
    0xA3u8 => op!(OpMnemonic::LAX, AddressingMode::IndirectX  , 6, Cpu::ilda, true),
    0xA7u8 => op!(OpMnemonic::LAX, AddressingMode::ZeroPage   , 3, Cpu::ilda, true),
    0xAFu8 => op!(OpMnemonic::LAX, AddressingMode::Absolute   , 4, Cpu::ilda, true),
    0xB3u8 => op!(OpMnemonic::LAX, AddressingMode::IndirectY  , 5, Cpu::ilda, true),
    0xB7u8 => op!(OpMnemonic::LAX, AddressingMode::ZeroPageY  , 4, Cpu::ilda, true),
    0xBFu8 => op!(OpMnemonic::LAX, AddressingMode::AbsoluteY  , 4, Cpu::ilda, true),
    0x85u8 => op!(OpMnemonic::STA, AddressingMode::ZeroPage   , 3, Cpu::sta, false),
    0x95u8 => op!(OpMnemonic::STA, AddressingMode::ZeroPageX  , 4, Cpu::sta, false),
    0x8Du8 => op!(OpMnemonic::STA, AddressingMode::Absolute   , 4, Cpu::sta, false),
    0x9Du8 => op!(OpMnemonic::STA, AddressingMode::AbsoluteX  , 5, Cpu::sta, false),
    0x99u8 => op!(OpMnemonic::STA, AddressingMode::AbsoluteY  , 5, Cpu::sta, false),
    0x81u8 => op!(OpMnemonic::STA, AddressingMode::IndirectX  , 6, Cpu::sta, false),
    0x91u8 => op!(OpMnemonic::STA, AddressingMode::IndirectY  , 6, Cpu::sta, false),
    0xA2u8 => op!(OpMnemonic::LDX, AddressingMode::Immediate  , 2, Cpu::ldx, false),
    0xA6u8 => op!(OpMnemonic::LDX, AddressingMode::ZeroPage   , 3, Cpu::ldx, false),
    0xB6u8 => op!(OpMnemonic::LDX, AddressingMode::ZeroPageY  , 4, Cpu::ldx, false),
    0xAEu8 => op!(OpMnemonic::LDX, AddressingMode::Absolute   , 4, Cpu::ldx, false),
    0xBEu8 => op!(OpMnemonic::LDX, AddressingMode::AbsoluteY  , 4, Cpu::ldx, false),
    0x86u8 => op!(OpMnemonic::STX, AddressingMode::ZeroPage   , 3, Cpu::stx, false),
    0x96u8 => op!(OpMnemonic::STX, AddressingMode::ZeroPageY  , 4, Cpu::stx, false),
    0x8Eu8 => op!(OpMnemonic::STX, AddressingMode::Absolute   , 4, Cpu::stx, false),
    0xA0u8 => op!(OpMnemonic::LDY, AddressingMode::Immediate  , 2, Cpu::ldy, false),
    0xA4u8 => op!(OpMnemonic::LDY, AddressingMode::ZeroPage   , 3, Cpu::ldy, false),
    0xB4u8 => op!(OpMnemonic::LDY, AddressingMode::ZeroPageX  , 4, Cpu::ldy, false),
    0xACu8 => op!(OpMnemonic::LDY, AddressingMode::Absolute   , 4, Cpu::ldy, false),
    0xBCu8 => op!(OpMnemonic::LDY, AddressingMode::AbsoluteX  , 4, Cpu::ldy, false),
    0x84u8 => op!(OpMnemonic::STY, AddressingMode::ZeroPage   , 3, Cpu::sty, false),
    0x94u8 => op!(OpMnemonic::STY, AddressingMode::ZeroPageX  , 4, Cpu::sty, false),
    0x8Cu8 => op!(OpMnemonic::STY, AddressingMode::Absolute   , 4, Cpu::sty, false),
    0xAAu8 => op!(OpMnemonic::TAX, AddressingMode::Implicid   , 2, Cpu::tax, false),
    0x8Au8 => op!(OpMnemonic::TXA, AddressingMode::Implicid   , 2, Cpu::txa, false),
    0xA8u8 => op!(OpMnemonic::TAY, AddressingMode::Implicid   , 2, Cpu::tay, false),
    0x98u8 => op!(OpMnemonic::TYA, AddressingMode::Implicid   , 2, Cpu::tya, false),
    0x69u8 => op!(OpMnemonic::ADC, AddressingMode::Immediate  , 2, Cpu::adc, false),
    0x65u8 => op!(OpMnemonic::ADC, AddressingMode::ZeroPage   , 3, Cpu::adc, false),
    0x75u8 => op!(OpMnemonic::ADC, AddressingMode::ZeroPageX  , 4, Cpu::adc, false),
    0x6Du8 => op!(OpMnemonic::ADC, AddressingMode::Absolute   , 4, Cpu::adc, false),
    0x7Du8 => op!(OpMnemonic::ADC, AddressingMode::AbsoluteX  , 4, Cpu::adc, false),
    0x79u8 => op!(OpMnemonic::ADC, AddressingMode::AbsoluteY  , 4, Cpu::adc, false),
    0x61u8 => op!(OpMnemonic::ADC, AddressingMode::IndirectX  , 6, Cpu::adc, false),
    0x71u8 => op!(OpMnemonic::ADC, AddressingMode::IndirectY  , 5, Cpu::adc, false),
    0xE9u8 => op!(OpMnemonic::SBC, AddressingMode::Immediate  , 2, Cpu::sbc, false),
    0xE5u8 => op!(OpMnemonic::SBC, AddressingMode::ZeroPage   , 3, Cpu::sbc, false),
    0xF5u8 => op!(OpMnemonic::SBC, AddressingMode::ZeroPageX  , 4, Cpu::sbc, false),
    0xEDu8 => op!(OpMnemonic::SBC, AddressingMode::Absolute   , 4, Cpu::sbc, false),
    0xFDu8 => op!(OpMnemonic::SBC, AddressingMode::AbsoluteX  , 4, Cpu::sbc, false),
    0xF9u8 => op!(OpMnemonic::SBC, AddressingMode::AbsoluteY  , 4, Cpu::sbc, false),
    0xE1u8 => op!(OpMnemonic::SBC, AddressingMode::IndirectX  , 6, Cpu::sbc, false),
    0xF1u8 => op!(OpMnemonic::SBC, AddressingMode::IndirectY  , 5, Cpu::sbc, false),
    0xE6u8 => op!(OpMnemonic::INC, AddressingMode::ZeroPage   , 5, Cpu::inc, false),
    0xF6u8 => op!(OpMnemonic::INC, AddressingMode::ZeroPageX  , 6, Cpu::inc, false),
    0xEEu8 => op!(OpMnemonic::INC, AddressingMode::Absolute   , 6, Cpu::inc, false),
    0xFEu8 => op!(OpMnemonic::INC, AddressingMode::AbsoluteX  , 7, Cpu::inc, false),
    0xC6u8 => op!(OpMnemonic::DEC, AddressingMode::ZeroPage   , 5, Cpu::dec, false),
    0xD6u8 => op!(OpMnemonic::DEC, AddressingMode::ZeroPageX  , 6, Cpu::dec, false),
    0xCEu8 => op!(OpMnemonic::DEC, AddressingMode::Absolute   , 6, Cpu::dec, false),
    0xDEu8 => op!(OpMnemonic::DEC, AddressingMode::AbsoluteX  , 7, Cpu::dec, false),
    0xE8u8 => op!(OpMnemonic::INX, AddressingMode::Implicid   , 2, Cpu::inx, false),
    0xCAu8 => op!(OpMnemonic::DEX, AddressingMode::Implicid   , 2, Cpu::dex, false),
    0xC8u8 => op!(OpMnemonic::INY, AddressingMode::Implicid   , 2, Cpu::iny, false),
    0x88u8 => op!(OpMnemonic::DEY, AddressingMode::Implicid   , 2, Cpu::dey, false),
    0x0Au8 => op!(OpMnemonic::ASL, AddressingMode::Accumulator, 2, Cpu::asl, false),
    0x06u8 => op!(OpMnemonic::ASL, AddressingMode::ZeroPage   , 5, Cpu::asl, false),
    0x16u8 => op!(OpMnemonic::ASL, AddressingMode::ZeroPageX  , 6, Cpu::asl, false),
    0x0Eu8 => op!(OpMnemonic::ASL, AddressingMode::Absolute   , 6, Cpu::asl, false),
    0x1Eu8 => op!(OpMnemonic::ASL, AddressingMode::AbsoluteX  , 7, Cpu::asl, false),
    0x4Au8 => op!(OpMnemonic::LSR, AddressingMode::Accumulator, 2, Cpu::lsr, false),
    0x46u8 => op!(OpMnemonic::LSR, AddressingMode::ZeroPage   , 5, Cpu::lsr, false),
    0x56u8 => op!(OpMnemonic::LSR, AddressingMode::ZeroPageX  , 6, Cpu::lsr, false),
    0x4Eu8 => op!(OpMnemonic::LSR, AddressingMode::Absolute   , 6, Cpu::lsr, false),
    0x5Eu8 => op!(OpMnemonic::LSR, AddressingMode::AbsoluteX  , 7, Cpu::lsr, false),
    0x2Au8 => op!(OpMnemonic::ROL, AddressingMode::Accumulator, 2, Cpu::rol, false),
    0x26u8 => op!(OpMnemonic::ROL, AddressingMode::ZeroPage   , 5, Cpu::rol, false),
    0x36u8 => op!(OpMnemonic::ROL, AddressingMode::ZeroPageX  , 6, Cpu::rol, false),
    0x2Eu8 => op!(OpMnemonic::ROL, AddressingMode::Absolute   , 6, Cpu::rol, false),
    0x3Eu8 => op!(OpMnemonic::ROL, AddressingMode::AbsoluteX  , 7, Cpu::rol, false),
    0x6Au8 => op!(OpMnemonic::ROR, AddressingMode::Accumulator, 2, Cpu::ror, false),
    0x66u8 => op!(OpMnemonic::ROR, AddressingMode::ZeroPage   , 5, Cpu::ror, false),
    0x76u8 => op!(OpMnemonic::ROR, AddressingMode::ZeroPageX  , 6, Cpu::ror, false),
    0x6Eu8 => op!(OpMnemonic::ROR, AddressingMode::Absolute   , 6, Cpu::ror, false),
    0x7Eu8 => op!(OpMnemonic::ROR, AddressingMode::AbsoluteX  , 7, Cpu::ror, false),
    0x29u8 => op!(OpMnemonic::AND, AddressingMode::Immediate  , 2, Cpu::and, false),
    0x25u8 => op!(OpMnemonic::AND, AddressingMode::ZeroPage   , 3, Cpu::and, false),
    0x35u8 => op!(OpMnemonic::AND, AddressingMode::ZeroPageX  , 4, Cpu::and, false),
    0x2Du8 => op!(OpMnemonic::AND, AddressingMode::Absolute   , 4, Cpu::and, false),
    0x3Du8 => op!(OpMnemonic::AND, AddressingMode::AbsoluteX  , 4, Cpu::and, false),
    0x39u8 => op!(OpMnemonic::AND, AddressingMode::AbsoluteY  , 4, Cpu::and, false),
    0x21u8 => op!(OpMnemonic::AND, AddressingMode::IndirectX  , 6, Cpu::and, false),
    0x31u8 => op!(OpMnemonic::AND, AddressingMode::IndirectY  , 5, Cpu::and, false),
    0x09u8 => op!(OpMnemonic::ORA, AddressingMode::Immediate  , 2, Cpu::ora, false),
    0x05u8 => op!(OpMnemonic::ORA, AddressingMode::ZeroPage   , 3, Cpu::ora, false),
    0x15u8 => op!(OpMnemonic::ORA, AddressingMode::ZeroPageX  , 4, Cpu::ora, false),
    0x0Du8 => op!(OpMnemonic::ORA, AddressingMode::Absolute   , 4, Cpu::ora, false),
    0x1Du8 => op!(OpMnemonic::ORA, AddressingMode::AbsoluteX  , 4, Cpu::ora, false),
    0x19u8 => op!(OpMnemonic::ORA, AddressingMode::AbsoluteY  , 4, Cpu::ora, false),
    0x01u8 => op!(OpMnemonic::ORA, AddressingMode::IndirectX  , 6, Cpu::ora, false),
    0x11u8 => op!(OpMnemonic::ORA, AddressingMode::IndirectY  , 5, Cpu::ora, false),
    0x49u8 => op!(OpMnemonic::EOR, AddressingMode::Immediate  , 2, Cpu::eor, false),
    0x45u8 => op!(OpMnemonic::EOR, AddressingMode::ZeroPage   , 3, Cpu::eor, false),
    0x55u8 => op!(OpMnemonic::EOR, AddressingMode::ZeroPageX  , 4, Cpu::eor, false),
    0x4Du8 => op!(OpMnemonic::EOR, AddressingMode::Absolute   , 4, Cpu::eor, false),
    0x5Du8 => op!(OpMnemonic::EOR, AddressingMode::AbsoluteX  , 4, Cpu::eor, false),
    0x59u8 => op!(OpMnemonic::EOR, AddressingMode::AbsoluteY  , 4, Cpu::eor, false),
    0x41u8 => op!(OpMnemonic::EOR, AddressingMode::IndirectX  , 6, Cpu::eor, false),
    0x51u8 => op!(OpMnemonic::EOR, AddressingMode::IndirectY  , 5, Cpu::eor, false),
    0x24u8 => op!(OpMnemonic::BIT, AddressingMode::ZeroPage   , 3, Cpu::bit, false),
    0x2Cu8 => op!(OpMnemonic::BIT, AddressingMode::Absolute   , 4, Cpu::bit, false),
    0xC9u8 => op!(OpMnemonic::CMP, AddressingMode::Immediate  , 2, Cpu::cmp, false),
    0xC5u8 => op!(OpMnemonic::CMP, AddressingMode::ZeroPage   , 3, Cpu::cmp, false),
    0xD5u8 => op!(OpMnemonic::CMP, AddressingMode::ZeroPageX  , 4, Cpu::cmp, false),
    0xCDu8 => op!(OpMnemonic::CMP, AddressingMode::Absolute   , 4, Cpu::cmp, false),
    0xDDu8 => op!(OpMnemonic::CMP, AddressingMode::AbsoluteX  , 4, Cpu::cmp, false),
    0xD9u8 => op!(OpMnemonic::CMP, AddressingMode::AbsoluteY  , 4, Cpu::cmp, false),
    0xC1u8 => op!(OpMnemonic::CMP, AddressingMode::IndirectX  , 6, Cpu::cmp, false),
    0xD1u8 => op!(OpMnemonic::CMP, AddressingMode::IndirectY  , 5, Cpu::cmp, false),
    0xE0u8 => op!(OpMnemonic::CPX, AddressingMode::Immediate  , 2, Cpu::cpx, false),
    0xE4u8 => op!(OpMnemonic::CPX, AddressingMode::ZeroPage   , 3, Cpu::cpx, false),
    0xECu8 => op!(OpMnemonic::CPX, AddressingMode::Absolute   , 4, Cpu::cpx, false),
    0xC0u8 => op!(OpMnemonic::CPY, AddressingMode::Immediate  , 2, Cpu::cpy, false),
    0xC4u8 => op!(OpMnemonic::CPY, AddressingMode::ZeroPage   , 3, Cpu::cpy, false),
    0xCCu8 => op!(OpMnemonic::CPY, AddressingMode::Absolute   , 4, Cpu::cpy, false),
    0x90u8 => op!(OpMnemonic::BCC, AddressingMode::Relative   , 2, Cpu::bcc, false),
    0xB0u8 => op!(OpMnemonic::BCS, AddressingMode::Relative   , 2, Cpu::bcs, false),
    0xF0u8 => op!(OpMnemonic::BEQ, AddressingMode::Relative   , 2, Cpu::beq, false),
    0xD0u8 => op!(OpMnemonic::BNE, AddressingMode::Relative   , 2, Cpu::bne, false),
    0x10u8 => op!(OpMnemonic::BPL, AddressingMode::Relative   , 2, Cpu::bpl, false),
    0x30u8 => op!(OpMnemonic::BMI, AddressingMode::Relative   , 2, Cpu::bmi, false),
    0x50u8 => op!(OpMnemonic::BVC, AddressingMode::Relative   , 2, Cpu::bvc, false),
    0x70u8 => op!(OpMnemonic::BVS, AddressingMode::Relative   , 2, Cpu::bvs, false),
    0x4Cu8 => op!(OpMnemonic::JMP, AddressingMode::Absolute   , 3, Cpu::jmp, false),
    0x6Cu8 => op!(OpMnemonic::JMP, AddressingMode::Indirect   , 5, Cpu::jmp, false),
    0x20u8 => op!(OpMnemonic::JSR, AddressingMode::Absolute   , 6, Cpu::jsr, false),
    0x60u8 => op!(OpMnemonic::RTS, AddressingMode::Implicid   , 6, Cpu::rts, false),
    // 0xu8 => op!(OpMnemonic::BRK, AddressingMode::Immediate , 0, Cpu::brk, false),
    0x40u8 => op!(OpMnemonic::RTI, AddressingMode::Implicid   , 6, Cpu::rti, false),
    0x48u8 => op!(OpMnemonic::PHA, AddressingMode::Implicid   , 3, Cpu::pha, false),
    0x68u8 => op!(OpMnemonic::PLA, AddressingMode::Implicid   , 4, Cpu::pla, false),
    0x08u8 => op!(OpMnemonic::PHP, AddressingMode::Implicid   , 3, Cpu::php, false),
    0x28u8 => op!(OpMnemonic::PLP, AddressingMode::Implicid   , 4, Cpu::plp, false),
    0x9Au8 => op!(OpMnemonic::TXS, AddressingMode::Implicid   , 2, Cpu::txs, false),
    0xBAu8 => op!(OpMnemonic::TSX, AddressingMode::Implicid   , 2, Cpu::tsx, false),
    0x18u8 => op!(OpMnemonic::CLC, AddressingMode::Implicid   , 2, Cpu::clc, false),
    0x38u8 => op!(OpMnemonic::SEC, AddressingMode::Implicid   , 2, Cpu::sec, false),
    // 0xu8 => op!(OpMnemonic::CLI, AddressingMode::Immediate , 0, Cpu::cli, false),
    0x78u8 => op!(OpMnemonic::SEI, AddressingMode::Implicid   , 2, Cpu::sei, false),
    0xD8u8 => op!(OpMnemonic::CLD, AddressingMode::Implicid   , 2, Cpu::cld, false),
    0xF8u8 => op!(OpMnemonic::SED, AddressingMode::Implicid   , 2, Cpu::sed, false),
    0xB8u8 => op!(OpMnemonic::CLV, AddressingMode::Implicid   , 2, Cpu::clv, false),
    0xEAu8 => op!(OpMnemonic::NOP, AddressingMode::Implicid   , 2, Cpu::nop, false),
    0x04u8 |
    0x44u8 |
    0x64u8 => op!(OpMnemonic::NOP, AddressingMode::ZeroPage   , 3, Cpu::inop, true),
    0x0Cu8 => op!(OpMnemonic::NOP, AddressingMode::Absolute   , 4, Cpu::inop, true),
    0x14u8 |
    0x34u8 |
    0x54u8 |
    0x74u8 |
    0xD4u8 |
    0xF4u8 => op!(OpMnemonic::NOP, AddressingMode::ZeroPageX  , 4, Cpu::inop, true),
    0x1Au8 |
    0x3Au8 |
    0x5Au8 |
    0x7Au8 |
    0xDAu8 |
    0xFAu8 => op!(OpMnemonic::NOP, AddressingMode::Implicid   , 2, Cpu::inop, true),
    0x80u8 |
    0x82u8 |
    0x89u8 |
    0xC2u8 |
    0xE2u8 => op!(OpMnemonic::NOP, AddressingMode::Immediate  , 2, Cpu::inop, true),
    0x1Cu8 |
    0x3Cu8 |
    0x5Cu8 |
    0x7Cu8 |
    0xDCu8 |
    0xFCu8 => op!(OpMnemonic::NOP, AddressingMode::AbsoluteX  , 4, Cpu::inop, true),
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
                log.push_str(" [ACTUAL LOG]");
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
                self.pc += 1;
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

    fn ilda(cpu: &mut Cpu, bus: &mut Bus, mode: AddressingMode, operands: &[u8]) -> u8 {
        let (value, page_crossed) = cpu.read_operand(bus, mode, operands);
        cpu.a = value;
        cpu.update_nz(cpu.a);
        Cpu::tax(cpu, bus, mode, operands);
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

    fn ldy(cpu: &mut Cpu, bus: &mut Bus, mode: AddressingMode, operands: &[u8]) -> u8 {
        let (value, page_crossed) = cpu.read_operand(bus, mode, operands);
        cpu.y = value;
        cpu.update_nz(cpu.y);
        if page_crossed { 1 } else { 0 }
    }

    fn sty(cpu: &mut Cpu, bus: &mut Bus, mode: AddressingMode, operands: &[u8]) -> u8 {
        cpu.write_operand(bus, mode, operands, cpu.y);
        0
    }

    fn tax(cpu: &mut Cpu, _bus: &mut Bus, _mode: AddressingMode, _operands: &[u8]) -> u8 {
        cpu.x = cpu.a;
        cpu.update_nz(cpu.x);
        0
    }

    fn txa(cpu: &mut Cpu, _bus: &mut Bus, _mode: AddressingMode, _operands: &[u8]) -> u8 {
        cpu.a = cpu.x;
        cpu.update_nz(cpu.a);
        0
    }

    fn tay(cpu: &mut Cpu, _bus: &mut Bus, _mode: AddressingMode, _operands: &[u8]) -> u8 {
        cpu.y = cpu.a;
        cpu.update_nz(cpu.y);
        0
    }

    fn tya(cpu: &mut Cpu, _bus: &mut Bus, _mode: AddressingMode, _operands: &[u8]) -> u8 {
        cpu.a = cpu.y;
        cpu.update_nz(cpu.a);
        0
    }

    fn adc(cpu: &mut Cpu, bus: &mut Bus, mode: AddressingMode, operands: &[u8]) -> u8 {
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

    fn sbc(cpu: &mut Cpu, bus: &mut Bus, mode: AddressingMode, operands: &[u8]) -> u8 {
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

    fn inc(cpu: &mut Cpu, bus: &mut Bus, mode: AddressingMode, operands: &[u8]) -> u8 {
        let (value, _) = cpu.read_operand(bus, mode, operands);
        let result = value.wrapping_add(1);
        cpu.write_operand(bus, mode, operands, result);
        cpu.update_nz(result);
        0
    }

    fn dec(cpu: &mut Cpu, bus: &mut Bus, mode: AddressingMode, operands: &[u8]) -> u8 {
        let (value, _) = cpu.read_operand(bus, mode, operands);
        let result = value.wrapping_sub(1);
        cpu.write_operand(bus, mode, operands, result);
        cpu.update_nz(result);
        0
    }

    fn inx(cpu: &mut Cpu, _bus: &mut Bus, _mode: AddressingMode, _operands: &[u8]) -> u8 {
        cpu.x = cpu.x.wrapping_add(1);
        cpu.update_nz(cpu.x);
        0
    }

    fn dex(cpu: &mut Cpu, _bus: &mut Bus, _mode: AddressingMode, _operands: &[u8]) -> u8 {
        cpu.x = cpu.x.wrapping_sub(1);
        cpu.update_nz(cpu.x);
        0
    }

    fn iny(cpu: &mut Cpu, _bus: &mut Bus, _mode: AddressingMode, _operands: &[u8]) -> u8 {
        cpu.y = cpu.y.wrapping_add(1);
        cpu.update_nz(cpu.y);
        0
    }

    fn dey(cpu: &mut Cpu, _bus: &mut Bus, _mode: AddressingMode, _operands: &[u8]) -> u8 {
        cpu.y = cpu.y.wrapping_sub(1);
        cpu.update_nz(cpu.y);
        0
    }

    fn asl(cpu: &mut Cpu, bus: &mut Bus, mode: AddressingMode, operands: &[u8]) -> u8 {
        let (value, _) = cpu.read_operand(bus, mode, operands);
        let result = value << 1;
        cpu.p.set(Flags::C, (value & 0b1000_0000) != 0);
        cpu.p.set(Flags::Z, result == 0);
        cpu.p.set(Flags::N, (result & 0b1000_0000) != 0);
        cpu.write_operand(bus, mode, operands, result);
        0
    }

    fn lsr(cpu: &mut Cpu, bus: &mut Bus, mode: AddressingMode, operands: &[u8]) -> u8 {
        let (value, _) = cpu.read_operand(bus, mode, operands);
        let result = value >> 1;
        cpu.p.set(Flags::C, (value & 0b1) != 0);
        cpu.p.set(Flags::Z, result == 0);
        cpu.p.set(Flags::N, false);
        cpu.write_operand(bus, mode, operands, result);
        0
    }

    fn rol(cpu: &mut Cpu, bus: &mut Bus, mode: AddressingMode, operands: &[u8]) -> u8 {
        let (value, _) = cpu.read_operand(bus, mode, operands);
        let carry = if cpu.p.contains(Flags::C) { 1 } else { 0 };
        let result = (value << 1) | carry;
        cpu.p.set(Flags::C, ((value >> 7) & 1) != 0);
        cpu.p.set(Flags::Z, result == 0);
        cpu.p.set(Flags::N, (result & 0x80) != 0);
        cpu.write_operand(bus, mode, operands, result);
        0
    }

    fn ror(cpu: &mut Cpu, bus: &mut Bus, mode: AddressingMode, operands: &[u8]) -> u8 {
        let (value, _) = cpu.read_operand(bus, mode, operands);
        let carry = if cpu.p.contains(Flags::C) { 1 } else { 0 };
        let result = (value >> 1) | (carry << 7);
        cpu.p.set(Flags::C, (value & 0b1) != 0);
        cpu.p.set(Flags::Z, result == 0);
        cpu.p.set(Flags::N, (result & 0b1000_0000) != 0);
        cpu.write_operand(bus, mode, operands, result);
        0
    }

    fn and(cpu: &mut Cpu, bus: &mut Bus, mode: AddressingMode, operands: &[u8]) -> u8 {
        let (value, page_crossed) = cpu.read_operand(bus, mode, operands);
        cpu.a &= value;
        cpu.update_nz(cpu.a);
        if page_crossed { 1 } else { 0 }
    }

    fn ora(cpu: &mut Cpu, bus: &mut Bus, mode: AddressingMode, operands: &[u8]) -> u8 {
        let (value, page_crossed) = cpu.read_operand(bus, mode, operands);
        cpu.a |= value;
        cpu.update_nz(cpu.a);
        if page_crossed { 1 } else { 0 }
    }

    fn eor(cpu: &mut Cpu, bus: &mut Bus, mode: AddressingMode, operands: &[u8]) -> u8 {
        let (value, page_crossed) = cpu.read_operand(bus, mode, operands);
        cpu.a ^= value;
        cpu.update_nz(cpu.a);
        if page_crossed { 1 } else { 0 }
    }

    fn bit(cpu: &mut Cpu, bus: &mut Bus, mode: AddressingMode, operands: &[u8]) -> u8 {
        let (value, _) = cpu.read_operand(bus, mode, operands);
        let result = value & cpu.a;
        cpu.p.set(Flags::Z, result == 0);
        cpu.p.set(Flags::V, value & 0b0100_0000 != 0);
        cpu.p.set(Flags::N, value & 0b1000_0000 != 0);
        0
    }

    fn cmp(cpu: &mut Cpu, bus: &mut Bus, mode: AddressingMode, operands: &[u8]) -> u8 {
        let (value, page_crossed) = cpu.read_operand(bus, mode, operands);
        let result = cpu.a.wrapping_sub(value);
        cpu.p.set(Flags::C, cpu.a >= value);
        cpu.p.set(Flags::Z, cpu.a == value);
        cpu.p.set(Flags::N, result & 0b1000_0000 != 0);
        if page_crossed { 1 } else { 0 }
    }

    fn cpx(cpu: &mut Cpu, bus: &mut Bus, mode: AddressingMode, operands: &[u8]) -> u8 {
        let (value, _) = cpu.read_operand(bus, mode, operands);
        let result = cpu.x.wrapping_sub(value);
        cpu.p.set(Flags::C, cpu.x >= value);
        cpu.p.set(Flags::Z, cpu.x == value);
        cpu.p.set(Flags::N, result & 0b1000_0000 != 0);
        0
    }

    fn cpy(cpu: &mut Cpu, bus: &mut Bus, mode: AddressingMode, operands: &[u8]) -> u8 {
        let (value, _) = cpu.read_operand(bus, mode, operands);
        let result = cpu.y.wrapping_sub(value);
        cpu.p.set(Flags::C, cpu.y >= value);
        cpu.p.set(Flags::Z, cpu.y == value);
        cpu.p.set(Flags::N, result & 0b1000_0000 != 0);
        0
    }

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
        let pc = cpu.pc.wrapping_sub(1);
        cpu.push_stack(bus, (pc >> 8) as u8);
        cpu.push_stack(bus, pc as u8);
        if let OperandValue::Address(addr, _) = mode.resolve(cpu, bus, operands) {
            cpu.pc = addr;
        }
        0
    }

    fn rts(cpu: &mut Cpu, bus: &mut Bus, _mode: AddressingMode, _operands: &[u8]) -> u8 {
        let lo = cpu.pop_stack(bus);
        let hi = cpu.pop_stack(bus);
        cpu.pc = u16::from_le_bytes([lo, hi]).wrapping_add(1);
        0
    }

    // fn brk(cpu: &mut Cpu, bus: &mut Bus, mode: AddressingMode, operands: &[u8]) -> u8 {}

    fn rti(cpu: &mut Cpu, bus: &mut Bus, _mode: AddressingMode, _operands: &[u8]) -> u8 {
        let mut p = Flags::from_bits(cpu.pop_stack(bus)).unwrap();
        p.remove(Flags::B);
        p.insert(Flags::_1);
        cpu.p = p;
        let lo = cpu.pop_stack(bus);
        let hi = cpu.pop_stack(bus);
        cpu.pc = u16::from_le_bytes([lo, hi]);
        0
    }

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
        let p = cpu.p.clone() | Flags::B | Flags::_1;
        cpu.push_stack(bus, p.bits());
        0
    }

    fn plp(cpu: &mut Cpu, bus: &mut Bus, _mode: AddressingMode, _operands: &[u8]) -> u8 {
        let mut p = Flags::from_bits(cpu.pop_stack(bus)).unwrap();
        p.remove(Flags::B);
        p.insert(Flags::_1);
        cpu.p = p;
        0
    }

    fn txs(cpu: &mut Cpu, _bus: &mut Bus, _mode: AddressingMode, _operands: &[u8]) -> u8 {
        cpu.sp = cpu.x as usize;
        0
    }

    fn tsx(cpu: &mut Cpu, _bus: &mut Bus, _mode: AddressingMode, _operands: &[u8]) -> u8 {
        cpu.x = cpu.sp as u8;
        cpu.update_nz(cpu.x);
        0
    }

    fn clc(cpu: &mut Cpu, _bus: &mut Bus, _mode: AddressingMode, _operands: &[u8]) -> u8 {
        cpu.p.set(Flags::C, false);
        0
    }

    fn sec(cpu: &mut Cpu, _bus: &mut Bus, _mode: AddressingMode, _operands: &[u8]) -> u8 {
        cpu.p.set(Flags::C, true);
        0
    }

    // fn cli(cpu: &mut Cpu, bus: &mut Bus, mode: AddressingMode, operands: &[u8]) -> u8 {}

    fn sei(cpu: &mut Cpu, _bus: &mut Bus, _mode: AddressingMode, _operands: &[u8]) -> u8 {
        cpu.p.set(Flags::I, true);
        0
    }

    fn cld(cpu: &mut Cpu, _bus: &mut Bus, _mode: AddressingMode, _operands: &[u8]) -> u8 {
        cpu.p.set(Flags::D, false);
        0
    }

    fn sed(cpu: &mut Cpu, _bus: &mut Bus, _mode: AddressingMode, _operands: &[u8]) -> u8 {
        cpu.p.set(Flags::D, true);
        0
    }

    fn clv(cpu: &mut Cpu, _bus: &mut Bus, _mode: AddressingMode, _operands: &[u8]) -> u8 {
        cpu.p.set(Flags::V, false);
        0
    }

    fn nop(_cpu: &mut Cpu, _bus: &mut Bus, _mode: AddressingMode, _operands: &[u8]) -> u8 {
        0
    }

    fn inop(cpu: &mut Cpu, bus: &mut Bus, mode: AddressingMode, operands: &[u8]) -> u8 {
        let (_, page_crossed) = cpu.read_operand(bus, mode, operands);
        if page_crossed { 1 } else { 0 }
    }
}
