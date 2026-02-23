use argh::FromArgs;

#[derive(FromArgs, Clone, Default)]
/// Nintendo NES emulator
pub struct Args {
    /// path to the ROM (.nes)
    #[argh(positional)]
    pub rom: Option<String>,

    /// start paused
    #[argh(short = 'p', switch)]
    pub pause: bool,

    /// enable logging
    #[argh(short = 'l', switch)]
    pub log: bool,

    /// emulator version
    #[argh(short = 'v', switch)]
    pub version: bool,
}
