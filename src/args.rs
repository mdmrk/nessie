use clap::Parser;

#[derive(Parser, Debug, Clone)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[arg(help = "Path to the ROM (.nes)")]
    pub rom: Option<String>,

    #[arg(short, long, default_value_t = false, help = "Start in paused state")]
    pub pause: bool,
}
