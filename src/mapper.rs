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
            0 => MapperIcon::Nintendo,
            _ => unreachable!(),
        }
    }

    pub fn bytes(&self) -> &'static [u8] {
        macro_rules! icon_path {
            ($file:literal) => {
                include_bytes!(concat!("./icon/mappers/", $file))
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
    fn read_prg(&self, addr: u16) -> u8;
    fn write_prg(&mut self, addr: u16, value: u8);
    fn read_chr(&self, addr: u16) -> u8;
    fn write_chr(&mut self, addr: u16, value: u8);
    fn mirroring(&self) -> Mirroring;
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Mirroring {
    Horizontal,
    Vertical,
    SingleScreenLower,
    SingleScreenUpper,
    FourScreen,
}

pub struct Mapper0 {
    prg_rom: Vec<u8>,
    chr_rom: Vec<u8>,
    mirroring: Mirroring,
}

impl Mapper0 {
    pub fn new(prg_rom: Vec<u8>, chr_rom: Vec<u8>, mirroring: Mirroring) -> Self {
        Self {
            prg_rom,
            chr_rom,
            mirroring,
        }
    }
}

impl Mapper for Mapper0 {
    fn read_prg(&self, addr: u16) -> u8 {
        match addr {
            0x6000..=0x7FFF => 0,
            0x8000..=0xFFFF => self.prg_rom[(addr as usize - 0x8000) % self.prg_rom.len()],
            _ => 0,
        }
    }

    fn write_prg(&mut self, _addr: u16, _value: u8) {}

    fn read_chr(&self, addr: u16) -> u8 {
        self.chr_rom[addr as usize % self.chr_rom.len()]
    }

    fn write_chr(&mut self, addr: u16, value: u8) {
        if !self.chr_rom.is_empty() {
            self.chr_rom[addr as usize] = value;
        }
    }

    fn mirroring(&self) -> Mirroring {
        self.mirroring
    }
}
