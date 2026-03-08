use std::sync::LazyLock;

use argh::FromArgs;

static ARGS: LazyLock<Args> = LazyLock::new(argh::from_env);

pub fn get_args() -> &'static Args {
    &ARGS
}

#[derive(FromArgs, Clone, Default)]
/// Nintendo NES emulator and debugger
pub struct Args {
    /// path to the ROM (.nes)
    #[argh(positional)]
    pub rom: Option<String>,

    /// start paused
    #[argh(short = 'p', switch)]
    pub pause: bool,

    /// enable instruction logging
    #[argh(short = 'l', switch)]
    pub log: bool,

    /// print version and exit
    #[argh(short = 'v', switch)]
    pub version: bool,

    /// create cache, config files... on cwd
    #[argh(switch)]
    pub portable: bool,
}
