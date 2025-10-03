#[derive(Default)]
pub struct Ppu {
    pub scanline: usize,
    pub h_pixel: usize,
}

impl Ppu {
    pub fn new() -> Self {
        Self {
            scanline: 0,
            h_pixel: 21, // FIXME?: may be initialized at init state / reset
        }
    }

    pub fn step(&mut self, cycles: usize) {
        self.h_pixel += 3 * cycles;
        if self.h_pixel > 340 {
            self.scanline += 1;
        }
        self.h_pixel %= 341;
        self.scanline %= 262;
    }
}
