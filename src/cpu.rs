pub struct Cpu {
    pub a: u32,
}

impl Cpu {
    pub fn new() -> Self {
        Self { a: 0 }
    }

    pub fn step(&mut self) {}
}
