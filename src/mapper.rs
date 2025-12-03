use savefile::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MapperIcon {
    Bad,
    Bandai,
    Bitcorp,
    ColorDreams,
    Front,
    Generic,
    Homebrew,
    IremAve,
    Irem,
    Jaleco,
    JyCompany,
    Kaiser,
    Konami,
    Namco,
    Nintendo,
    Ntdec,
    PirateMmc3,
    Pirate,
    Rare,
    Sunsoft,
    Supertone,
    Taito,
    Tengen,
    Thq,
    Tools,
    Txc,
    WhirlwindManu,
}

impl MapperIcon {
    pub fn from_mapper_number(mapper_num: u8) -> Self {
        match mapper_num {
            0 | 1 => MapperIcon::Nintendo,
            _ => unreachable!(),
        }
    }

    pub fn bytes(&self) -> &'static [u8] {
        macro_rules! icon_path {
            ($file:literal) => {
                include_bytes!(concat!("../assets/icons/mappers/", $file))
            };
        }

        match self {
            MapperIcon::Bad => icon_path!("bad.png"),
            MapperIcon::Bandai => icon_path!("bandai.png"),
            MapperIcon::Bitcorp => icon_path!("bitcorp.png"),
            MapperIcon::ColorDreams => icon_path!("color_dreams.png"),
            MapperIcon::Front => icon_path!("front.png"),
            MapperIcon::Generic => icon_path!("generic.png"),
            MapperIcon::Homebrew => icon_path!("homebrew.png"),
            MapperIcon::IremAve => icon_path!("irem_ave.png"),
            MapperIcon::Irem => icon_path!("irem.png"),
            MapperIcon::Jaleco => icon_path!("jaleco.png"),
            MapperIcon::JyCompany => icon_path!("jycompany.png"),
            MapperIcon::Kaiser => icon_path!("kaiser.png"),
            MapperIcon::Konami => icon_path!("konami.png"),
            MapperIcon::Namco => icon_path!("namco.png"),
            MapperIcon::Nintendo => icon_path!("nintendo.png"),
            MapperIcon::Ntdec => icon_path!("ntdec.png"),
            MapperIcon::PirateMmc3 => icon_path!("pirate_mmc3.png"),
            MapperIcon::Pirate => icon_path!("pirate.png"),
            MapperIcon::Rare => icon_path!("rare.png"),
            MapperIcon::Sunsoft => icon_path!("sunsoft.png"),
            MapperIcon::Supertone => icon_path!("supertone.png"),
            MapperIcon::Taito => icon_path!("taito.png"),
            MapperIcon::Tengen => icon_path!("tengen.png"),
            MapperIcon::Thq => icon_path!("thq.png"),
            MapperIcon::Tools => icon_path!("tools.png"),
            MapperIcon::Txc => icon_path!("txc.png"),
            MapperIcon::WhirlwindManu => icon_path!("whirlwind_manu.png"),
        }
    }
}

pub trait Mapper {
    fn read_prg(&self, addr: u16) -> Option<u8>;
    fn write_prg(&mut self, addr: u16, value: u8);
    fn read_chr(&self, addr: u16) -> u8;
    fn write_chr(&mut self, addr: u16, value: u8);
    fn mirroring(&self) -> Mirroring;
}

#[derive(Debug, Savefile, Clone)]
pub enum MapperEnum {
    Mapper0(Mapper0),
    Mapper1(Mapper1),
}

impl MapperEnum {
    pub fn read_prg(&self, addr: u16) -> Option<u8> {
        match self {
            MapperEnum::Mapper0(m) => m.read_prg(addr),
            MapperEnum::Mapper1(m) => m.read_prg(addr),
        }
    }
    pub fn write_prg(&mut self, addr: u16, value: u8) {
        match self {
            MapperEnum::Mapper0(m) => m.write_prg(addr, value),
            MapperEnum::Mapper1(m) => m.write_prg(addr, value),
        }
    }
    pub fn read_chr(&self, addr: u16) -> u8 {
        match self {
            MapperEnum::Mapper0(m) => m.read_chr(addr),
            MapperEnum::Mapper1(m) => m.read_chr(addr),
        }
    }
    pub fn write_chr(&mut self, addr: u16, value: u8) {
        match self {
            MapperEnum::Mapper0(m) => m.write_chr(addr, value),
            MapperEnum::Mapper1(m) => m.write_chr(addr, value),
        }
    }
    pub fn mirroring(&self) -> Mirroring {
        match self {
            MapperEnum::Mapper0(m) => m.mirroring(),
            MapperEnum::Mapper1(m) => m.mirroring(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Savefile)]
pub enum Mirroring {
    Horizontal,
    Vertical,
    SingleScreenLower,
    SingleScreenUpper,
    FourScreen,
}

#[derive(Clone, Debug, Savefile)]
pub struct Mapper0 {
    prg_rom: Vec<u8>,
    chr_mem: Vec<u8>, // ROM or RAM
    mirroring: Mirroring,
    is_chr_ram: bool,
}

impl Mapper0 {
    pub fn new(prg_rom: Vec<u8>, chr_rom: Vec<u8>, mirroring: Mirroring) -> Self {
        let is_chr_ram = chr_rom.is_empty();
        let chr_mem = if is_chr_ram { vec![0; 0x2000] } else { chr_rom };

        Self {
            prg_rom,
            chr_mem,
            mirroring,
            is_chr_ram,
        }
    }
}

impl Mapper for Mapper0 {
    fn read_prg(&self, addr: u16) -> Option<u8> {
        match addr {
            0x6000..=0x7FFF => None,
            0x8000..=0xFFFF => Some(self.prg_rom[(addr as usize - 0x8000) % self.prg_rom.len()]),
            _ => None,
        }
    }

    fn write_prg(&mut self, _addr: u16, _value: u8) {}

    fn read_chr(&self, addr: u16) -> u8 {
        self.chr_mem[(addr as usize) % self.chr_mem.len()]
    }

    fn write_chr(&mut self, addr: u16, value: u8) {
        if !self.is_chr_ram {
            return;
        }

        let index = (addr as usize) % self.chr_mem.len();
        self.chr_mem[index] = value;
    }

    fn mirroring(&self) -> Mirroring {
        self.mirroring
    }
}

#[derive(Clone, Debug, Savefile)]
pub struct Mapper1 {
    prg_rom: Vec<u8>,
    chr_mem: Vec<u8>, // ROM or RAM
    prg_ram: Vec<u8>,
    shift_register: u8,
    write_count: u8,
    control: u8,
    chr_bank_0: u8,
    chr_bank_1: u8,
    prg_bank: u8,
    is_chr_ram: bool,
}

impl Mapper1 {
    pub fn new(prg_rom: Vec<u8>, chr_rom: Vec<u8>, _mirroring: Mirroring) -> Self {
        let is_chr_ram = chr_rom.is_empty();
        let chr_mem = if is_chr_ram { vec![0; 0x2000] } else { chr_rom };

        Self {
            prg_rom,
            chr_mem,
            prg_ram: vec![0; 0x2000],
            shift_register: 0x10,
            write_count: 0,
            control: 0x0C,
            chr_bank_0: 0,
            chr_bank_1: 0,
            prg_bank: 0,
            is_chr_ram,
        }
    }
}

impl Mapper for Mapper1 {
    fn read_prg(&self, addr: u16) -> Option<u8> {
        match addr {
            0x6000..=0x7FFF => Some(self.prg_ram[(addr - 0x6000) as usize]),
            0x8000..=0xFFFF => {
                let prg_mode = (self.control >> 2) & 0x03;
                let prg_bank = self.prg_bank & 0x0F;
                let num_banks = (self.prg_rom.len() / 0x4000) as u8;

                let bank = match prg_mode {
                    0 | 1 => {
                        if addr < 0xC000 {
                            (prg_bank & 0xFE) as usize
                        } else {
                            ((prg_bank & 0xFE) + 1) as usize
                        }
                    }
                    2 => {
                        if addr < 0xC000 {
                            0
                        } else {
                            prg_bank as usize
                        }
                    }
                    3 => {
                        if addr < 0xC000 {
                            prg_bank as usize
                        } else {
                            (num_banks - 1) as usize
                        }
                    }
                    _ => unreachable!(),
                };

                let offset = ((addr & 0x3FFF) as usize) + (bank * 0x4000);
                Some(self.prg_rom[offset % self.prg_rom.len()])
            }
            _ => None,
        }
    }

    fn write_prg(&mut self, addr: u16, value: u8) {
        match addr {
            0x6000..=0x7FFF => {
                self.prg_ram[(addr - 0x6000) as usize] = value;
            }
            0x8000..=0xFFFF => {
                if value & 0x80 != 0 {
                    self.shift_register = 0x10;
                    self.write_count = 0;
                    self.control |= 0x0C;
                } else {
                    let bit = value & 0x01;
                    self.shift_register >>= 1;
                    self.shift_register |= bit << 4;
                    self.write_count += 1;

                    if self.write_count == 5 {
                        let reg_value = self.shift_register;

                        match addr {
                            0x8000..=0x9FFF => self.control = reg_value,
                            0xA000..=0xBFFF => self.chr_bank_0 = reg_value,
                            0xC000..=0xDFFF => self.chr_bank_1 = reg_value,
                            0xE000..=0xFFFF => self.prg_bank = reg_value,
                            _ => {}
                        }

                        self.shift_register = 0x10;
                        self.write_count = 0;
                    }
                }
            }
            _ => {}
        }
    }

    fn read_chr(&self, addr: u16) -> u8 {
        let chr_mode = (self.control >> 4) & 0x01;

        let bank = if chr_mode == 0 {
            if addr < 0x1000 {
                (self.chr_bank_0 & 0x1E) as usize
            } else {
                ((self.chr_bank_0 & 0x1E) + 1) as usize
            }
        } else if addr < 0x1000 {
            self.chr_bank_0 as usize
        } else {
            self.chr_bank_1 as usize
        };

        let bank_size = 0x1000;
        let offset = (addr as usize & (bank_size - 1)) + (bank * bank_size);

        self.chr_mem[offset % self.chr_mem.len()]
    }

    fn write_chr(&mut self, addr: u16, value: u8) {
        if !self.is_chr_ram {
            return;
        }

        let chr_mode = (self.control >> 4) & 0x01;

        let bank = if chr_mode == 0 {
            if addr < 0x1000 {
                (self.chr_bank_0 & 0x1E) as usize
            } else {
                ((self.chr_bank_0 & 0x1E) + 1) as usize
            }
        } else if addr < 0x1000 {
            self.chr_bank_0 as usize
        } else {
            self.chr_bank_1 as usize
        };

        let bank_size = 0x1000;
        let offset = (addr as usize & (bank_size - 1)) + (bank * bank_size);

        if offset < self.chr_mem.len() {
            self.chr_mem[offset] = value;
        }
    }

    fn mirroring(&self) -> Mirroring {
        match self.control & 0x03 {
            0 => Mirroring::SingleScreenLower,
            1 => Mirroring::SingleScreenUpper,
            2 => Mirroring::Vertical,
            3 => Mirroring::Horizontal,
            _ => unreachable!(),
        }
    }
}
