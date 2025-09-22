use bytesize::ByteSize;
use modular_bitfield::prelude::*;

use crate::mapper::MMC1;

#[derive(Clone, Copy, Debug, Specifier)]
pub enum NametableArrangement {
    Vertical = 0,
    Horitzontal = 1,
}

#[bitfield(bytes = 1)]
#[derive(Debug, Clone)]
pub struct Flags6 {
    pub nametable_arrangement: NametableArrangement,
    pub has_backed_prg_ram: bool,
    pub has_trainer: bool,
    pub has_alt_nametable_layout: bool,
    pub mapper_lower: B4,
}

#[bitfield(bytes = 1)]
#[derive(Debug, Clone)]
pub struct Flags7 {
    pub has_vs_unisystem: bool,
    pub has_playchoice_10: bool,
    pub this_is_two: B2,
    pub mapper_upper: B4,
}

#[derive(Clone, Copy, Debug, Specifier)]
pub enum TVSystem {
    NTSC = 0,
    PAL = 1,
}

#[bitfield(bytes = 1)]
#[derive(Debug, Clone)]
pub struct Flags9 {
    pub tv_system: TVSystem,
    pub reserved: B7,
}

#[derive(Clone, Copy, Debug, Specifier)]
#[bits = 2]
pub enum TVSystem2 {
    NTSC = 0,
    DualCompatible = 1,
    PAL = 2,
    DualCompatible2 = 3, // FIXME: 1 and 3 are dual compatible
}

#[bitfield(bytes = 1)]
#[derive(Debug, Clone)]
pub struct Flags10 {
    pub tv_system: TVSystem2,
    pub padding1: B2,
    pub prg_ram_present: bool,
    pub has_bus_conflicts: bool,
    pub padding2: B2,
}

#[repr(C)]
#[derive(Clone)]
pub struct Header {
    pub magic: [u8; 4],
    pub prg_rom_size: u8,
    pub chr_rom_size: u8,
    pub flags6: Flags6,
    pub flags7: Flags7,
    pub prg_ram_size: u8,
    pub flags9: Flags9,
    pub flags10: Flags10,
    _pad: [u8; 5],
}

impl Header {
    pub fn get_mapper(&self) -> u8 {
        self.flags7.mapper_upper() << 4 | self.flags6.mapper_lower()
    }
}

#[derive(Clone)]
pub struct Cart {
    pub header: Header,
    pub rom: Vec<u8>,
    pub prg_data: Vec<u8>,
    pub mapper: MMC1,
}

impl Cart {
    pub fn insert(rom_path: &String) -> Option<Self> {
        match std::fs::read(rom_path) {
            Ok(contents) => {
                let header = unsafe { std::ptr::read(contents.as_ptr() as *const Header) };
                let rom = contents.clone();
                let prg_data_size = 16 * 1024 * header.prg_rom_size as usize;
                let prg_data_offset = if !header.flags6.has_trainer() {
                    size_of::<Header>() + 512
                } else {
                    size_of::<Header>()
                };
                let prg_data = rom[prg_data_offset..prg_data_offset + prg_data_size].to_vec();

                Some(Self {
                    header,
                    rom,
                    prg_data,
                    mapper: MMC1::new(),
                })
            }
            Err(_) => None,
        }
    }
}
