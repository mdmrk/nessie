use log::error;
use modular_bitfield::prelude::*;
use sha1_smol::Sha1;

use crate::mapper::{Mapper, Mapper0, Mapper1, Mirroring};

#[derive(Clone, Copy, Debug, Specifier, PartialEq)]
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
    DualCompatible2 = 3,
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
#[derive(Clone, Debug)]
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
    pub fn mapper_number(&self) -> u8 {
        self.flags7.mapper_upper() << 4 | self.flags6.mapper_lower()
    }

    pub fn make_mapper(
        &self,
        prg_rom: Vec<u8>,
        chr_rom: Vec<u8>,
        mirroring: Mirroring,
    ) -> Box<dyn Mapper> {
        let mapper_num = self.mapper_number();

        match mapper_num {
            0 => Box::new(Mapper0::new(prg_rom, chr_rom, mirroring)),
            1 => Box::new(Mapper1::new(prg_rom, chr_rom, mirroring)),
            _ => panic!("Unsupported mapper ({})", mapper_num),
        }
    }
}

pub struct Cart {
    pub header: Header,
    pub rom: Vec<u8>,
    pub mapper: Box<dyn Mapper>,
    pub hash: String,
}

impl Cart {
    pub fn from_bytes(contents: Vec<u8>) -> Option<Self> {
        let mut hasher = Sha1::new();
        hasher.update(&contents);
        let hash = hasher.digest().to_string();
        let header = unsafe { std::ptr::read(contents.as_ptr() as *const Header) };
        let header_magic = [0x4E, 0x45, 0x53, 0x1A];
        if header.magic != header_magic {
            error!("Wrong ROM magic number");
            return None;
        }
        let rom = contents.clone();
        let prg_rom_size = 16 * 1024 * header.prg_rom_size as usize;
        let prg_rom_offset = if header.flags6.has_trainer() {
            size_of::<Header>() + 512
        } else {
            size_of::<Header>()
        };

        if prg_rom_offset + prg_rom_size > rom.len() {
            return None;
        }

        let prg_rom = rom[prg_rom_offset..prg_rom_offset + prg_rom_size].to_vec();
        let chr_rom_size = 8 * 1024 * header.chr_rom_size as usize;
        let chr_rom_offset = prg_rom_offset + prg_rom_size;

        let chr_rom = if chr_rom_offset + chr_rom_size <= rom.len() {
            rom[chr_rom_offset..chr_rom_offset + chr_rom_size].to_vec()
        } else {
            vec![0; chr_rom_size]
        };

        let mirroring = if header.flags6.nametable_arrangement() == NametableArrangement::Vertical {
            Mirroring::Vertical
        } else {
            Mirroring::Horizontal
        };
        let mapper = header.make_mapper(prg_rom, chr_rom, mirroring);

        Some(Self {
            header,
            rom,
            mapper,
            hash,
        })
    }

    pub fn insert(rom_path: &str) -> Option<Self> {
        match std::fs::read(rom_path) {
            Ok(contents) => Self::from_bytes(contents),
            Err(e) => {
                error!("{e}");
                None
            }
        }
    }
}

impl Clone for Cart {
    fn clone(&self) -> Self {
        Self {
            header: self.header.clone(),
            rom: self.rom.clone(),
            mapper: self.mapper.clone_mapper(),
            hash: self.hash.clone(),
        }
    }
}
