use packed_struct::prelude::*;
use size::Size;

#[derive(PrimitiveEnum_u8, Clone, Copy, Debug, PartialEq)]
pub enum NametableArrangement {
    Vertical = 0,
    Horitzontal = 1,
}

#[repr(C)]
#[derive(PackedStruct, Clone)]
#[packed_struct(bit_numbering = "msb0")]
pub struct Flags6 {
    #[packed_field(bits = "0", ty = "enum")]
    pub nametable_arrangement: NametableArrangement,
    #[packed_field(bits = "1")]
    pub has_backed_prg_ram: bool,
    #[packed_field(bits = "2")]
    pub has_trainer: bool,
    #[packed_field(bits = "3")]
    pub has_alt_nametable_layout: bool,
    #[packed_field(bits = "4..=7")]
    pub mapper: Integer<u8, packed_bits::Bits<4>>,
}

#[repr(C)]
#[derive(Clone)]
pub struct Header {
    pub magic: [u8; 4],
    pub prg_rom_size: u8,
    pub chr_rom_size: u8,
    pub flags6: Flags6,
    pub flags7: u8,  // TODO: Implement properly
    pub flags8: u8,  // TODO: Implement properly
    pub flags9: u8,  // TODO: Implement properly
    pub flags10: u8, // TODO: Implement properly
    pub _pad: [u8; 5],
}

#[derive(Clone)]
pub struct Cart {
    pub header: Header,
    pub rom: Vec<u8>,
    pub prg_data: *const [u8],
}

impl Cart {
    pub fn insert(rom_path: &String) -> Option<Self> {
        match std::fs::read(rom_path) {
            Ok(contents) => {
                let header = unsafe { std::ptr::read(contents.as_ptr() as *const Header) };
                let rom = contents.clone();
                let prg_data_size =
                    Size::from_kilobytes(16).bytes() as usize * header.prg_rom_size as usize;
                let prg_data = std::ptr::slice_from_raw_parts(
                    if header.flags6.has_trainer {
                        contents
                            .as_ptr()
                            .wrapping_add(size_of::<Header>())
                            .wrapping_add(Size::from_bytes(512).bytes() as usize)
                    } else {
                        contents.as_ptr().wrapping_add(size_of::<Header>())
                    },
                    prg_data_size,
                );

                Some(Self {
                    header,
                    rom,
                    prg_data,
                })
            }
            Err(_) => None,
        }
    }
}
