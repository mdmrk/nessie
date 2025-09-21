use bytesize::ByteSize;
use modular_bitfield::prelude::*;

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

#[repr(C)]
#[derive(Clone)]
pub struct Header {
    pub magic: [u8; 4],
    pub prg_rom_size: u8,
    pub chr_rom_size: u8,
    pub flags6: Flags6,
    pub flags7: Flags7,
    pub prg_ram_size: u8,
    pub flags9: u8,  // TODO: Implement properly
    pub flags10: u8, // TODO: Implement properly
    _pad: [u8; 5],
}

impl Header {
    pub fn get_mapper(&self) -> u8 {
        (self.flags7.mapper_upper() as u8) << 4 | self.flags6.mapper_lower()
    }
}

#[derive(Clone)]
pub struct Cart {
    pub header: Header,
    pub rom: Vec<u8>,
    pub prg_data_ptr: *const [u8],
}

impl Cart {
    pub fn insert(rom_path: &String) -> Option<Self> {
        match std::fs::read(rom_path) {
            Ok(contents) => {
                let header = unsafe { std::ptr::read(contents.as_ptr() as *const Header) };
                let rom = contents.clone();
                let prg_data_size = ByteSize::kib(16).0 as usize * header.prg_rom_size as usize;
                let prg_data_ptr = std::ptr::slice_from_raw_parts(
                    if !header.flags6.has_trainer() {
                        contents
                            .as_ptr()
                            .wrapping_add(size_of::<Header>())
                            .wrapping_add(ByteSize::b(512).0 as usize)
                    } else {
                        contents.as_ptr().wrapping_add(size_of::<Header>())
                    },
                    prg_data_size,
                );

                Some(Self {
                    header,
                    rom,
                    prg_data_ptr,
                })
            }
            Err(_) => None,
        }
    }
}
