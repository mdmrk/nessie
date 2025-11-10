use argh::FromArgs;

#[derive(FromArgs, Clone)]
/// NES emulator
pub struct Args {
    /// path to the ROM (.nes)
    #[argh(positional)]
    pub rom: Option<String>,

    /// start in paused state
    #[argh(short = 'p', switch)]
    pub pause: bool,

    /// a reference output log file to test roms
    #[argh(short = 'l', option)]
    pub logfile: Option<String>,
}
