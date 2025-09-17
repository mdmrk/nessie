use packed_struct::prelude::*;

#[derive(PrimitiveEnum_u8, Clone, Copy, Debug, PartialEq)]
pub enum NametableArrangement {
    Vertical = 0,
    Horitzontal = 1,
}

#[derive(PackedStruct)]
#[packed_struct(bit_numbering = "msb0")]
pub struct Flags6 {
    #[packed_field(bits = "0", ty = "enum")]
    nametable_arrangement: NametableArrangement,
    #[packed_field(bits = "1")]
    has_backed_prg_ram: bool,
    #[packed_field(bits = "2")]
    has_trainer: bool,
    #[packed_field(bits = "3")]
    has_alt_nametable_layout: bool,
    #[packed_field(bits = "4..=7")]
    mapper: Integer<u8, packed_bits::Bits<4>>,
}

struct Header {
    magic: [u8; 4],
    prg_rom_size: u8,
    chr_rom_size: u8,
    flags6: Flags6,
    flags7: u8,
    flags8: u8,
    flags9: u8,
    flags10: u8,
    _pad: [u8; 5],
}

pub struct Cart {
    // header: Header,
    rom: Vec<u8>,
}

impl Cart {
    pub fn insert(rom_path: &String) -> Option<Self> {
        match std::fs::read(rom_path) {
            Ok(content) => Some(Self {
                // header: *bytemuck::from_bytes(&content),
                rom: content,
            }),
            Err(_) => None,
        }
    }
}
