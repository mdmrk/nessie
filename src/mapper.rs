#[derive(Clone)]
pub struct MMC1 {
    bank: usize,
}

impl MMC1 {
    pub fn new() -> Self {
        Self { bank: 0 }
    }
}
