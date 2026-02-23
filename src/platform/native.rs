use std::fs;
use std::{
    path::PathBuf,
    sync::mpsc,
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use directories::ProjectDirs;
use egui::{Color32, Context as EguiContext};
use log::{error, info};
use rfd::FileDialog;
use ringbuf::{HeapProd, HeapRb, traits::Split};
use savefile::{load_file, save_file_compressed};

use crate::args::Args;
use crate::audio::Audio;
use crate::debug::DebugSnapshot;
use crate::emu::{Command, Emu, EmuState, Event};
use crate::platform::RomSource;
use crate::ppu::{FRAME_HEIGHT, FRAME_WIDTH};

pub struct PlatformRunner {
    pub command_tx: Option<mpsc::Sender<Command>>,
    pub event_rx: Option<mpsc::Receiver<Event>>,
    pub debug_rx: Option<triple_buffer::Output<DebugSnapshot>>,
    pub frame_rx: Option<triple_buffer::Output<Vec<Color32>>>,
    pub emu_thread_handle: Option<JoinHandle<()>>,
    pub audio: Option<Audio>,
    pub running: bool,
    pub paused: bool,
}

impl PlatformRunner {
    pub fn new() -> Self {
        Self {
            command_tx: None,
            event_rx: None,
            debug_rx: None,
            frame_rx: None,
            emu_thread_handle: None,
            audio: None,
            running: false,
            paused: false,
        }
    }

    pub fn start(&mut self, rom: RomSource, args: Option<Args>) {
        if self.running {
            self.stop();
        }

        let rb = HeapRb::<f32>::new(4096);
        let (tx, rx) = rb.split();
        let (audio_handle, sample_rate) = match Audio::new(rx) {
            Ok(audio) => {
                let rate = audio.sample_rate;
                (Some(audio), rate)
            }
            Err(e) => {
                error!("Failed to initialize audio: {}", e);
                (None, 44100.0)
            }
        };

        self.audio = audio_handle;
        let pause = args.pause;

        let (command_tx, command_rx) = mpsc::channel();
        let (event_rx_tx, event_rx) = mpsc::channel();
        let (debug_tx, debug_rx) = triple_buffer::triple_buffer(&DebugSnapshot::default());
        let (frame_tx, frame_rx) =
            triple_buffer::triple_buffer(&vec![Color32::BLACK; FRAME_WIDTH * FRAME_HEIGHT]);

        self.command_tx = Some(command_tx);
        self.event_rx = Some(event_rx);
        self.debug_rx = Some(debug_rx);
        self.frame_rx = Some(frame_rx);

        let event_tx_clone = event_rx_tx.clone();

        let handle = thread::Builder::new()
            .name("emu_thread".to_string())
            .spawn(move || {
                let result = emu_thread(
                    &args,
                    command_rx,
                    event_rx_tx,
                    debug_tx,
                    frame_tx,
                    rom,
                    tx,
                    sample_rate,
                );

                if let Err(e) = result {
                    let msg = if let Some(s) = e.downcast_ref::<&str>() {
                        s.to_string()
                    } else if let Some(s) = e.downcast_ref::<String>() {
                        s.clone()
                    } else {
                        "Unknown emulator error".to_string()
                    };

                    let _ = event_tx_clone.send(Event::Crashed(msg));
                }
            })
            .expect("Failed to spawn emu thread");

        self.emu_thread_handle = Some(handle);
        self.running = true;
        self.paused = pause;
    }

    pub fn stop(&mut self) {
        if let Some(command_tx) = self.command_tx.take() {
            let _ = command_tx.send(Command::Stop);

            if let Some(handle) = self.emu_thread_handle.take() {
                let _ = handle.join();
            }
        }
        self.event_rx = None;
        self.debug_rx = None;
        self.frame_rx = None;
        self.running = false;
        self.paused = false;
    }

    pub fn pause(&mut self) {
        if self.running && !self.paused {
            self.send_command(Command::Pause);
            self.paused = true;
        }
    }

    pub fn resume(&mut self) {
        if self.running && self.paused {
            self.send_command(Command::Resume);
            self.paused = false;
        }
    }

    pub fn step(&mut self) {
        if self.running && self.paused {
            self.send_command(Command::Step);
        }
    }

    pub fn send_command(&self, command: Command) {
        if let Some(command_tx) = &self.command_tx
            && let Err(e) = command_tx.send(command)
        {
            error!("{e}");
        }
    }

    pub fn handle_events(&mut self, _ctx: &EguiContext) -> Vec<Event> {
        let mut events = Vec::new();
        if let Some(rx) = &self.event_rx {
            while let Ok(event) = rx.try_recv() {
                events.push(event);
            }
        }
        events
    }

    pub fn get_debug_snapshot(&mut self) -> Option<DebugSnapshot> {
        self.debug_rx.as_mut().map(|rx| rx.read().clone())
    }

    pub fn get_frame_data(&mut self) -> Option<&[Color32]> {
        if let Some(rx) = &mut self.frame_rx {
            Some(rx.read())
        } else {
            None
        }
    }

    pub fn pick_rom(&mut self) -> Option<PathBuf> {
        FileDialog::new()
            .add_filter("NES rom", &["nes"])
            .pick_file()
    }

    pub fn pick_state_file(&self) {
        if let Ok(path) = get_project_dir(ProjDirKind::Cache) {
            let mut fd = FileDialog::new().add_filter("ROM state file", &["bin"]);
            if std::fs::exists(&path).is_ok_and(|f| f) {
                fd = fd.set_directory(path);
            }
            if let Some(state_path) = fd.pick_file() {
                self.send_command(Command::LoadState(state_path));
            }
        }
    }
}

impl Default for PlatformRunner {
    fn default() -> Self {
        Self::new()
    }
}

pub fn emu_thread(
    args: &Args,
    command_rx: mpsc::Receiver<Command>,
    event_tx: mpsc::Sender<Event>,
    debug_tx: triple_buffer::Input<DebugSnapshot>,
    frame_tx: triple_buffer::Input<Vec<Color32>>,
    rom: RomSource,
    audio_producer: HeapProd<f32>,
    sample_rate: f32,
) -> Result<()> {
    let mut emu = Emu::new(
        event_tx,
        debug_tx,
        frame_tx,
        args.log,
        audio_producer,
        sample_rate,
    );

    match rom {
        RomSource::Path(path) => emu.load_rom(path.to_str().unwrap())?,
        RomSource::Bytes(bytes) => emu.load_rom_from_bytes(bytes)?,
    }

    if args.pause {
        emu.pause();
    }

    let frame_duration = Duration::from_secs_f64(1.0 / 60.0);
    let mut frame_start_time = Instant::now();

    loop {
        while let Ok(command) = command_rx.try_recv() {
            match command {
                Command::Stop => {
                    emu.stop();
                }
                Command::Pause => {
                    emu.pause();
                }
                Command::Resume => {
                    emu.resume();
                }
                #[cfg(not(target_arch = "wasm32"))]
                Command::SaveState => {
                    save_state(&emu).unwrap_or_else(|e| error!("Failed to save state: {e}"));
                }
                #[cfg(not(target_arch = "wasm32"))]
                Command::LoadState(path) => {
                    load_state(&mut emu, &path)
                        .unwrap_or_else(|e| error!("Failed to load state: {e}"));
                }
                Command::Step => {
                    emu.want_step = true;
                }
                Command::MemoryAddress(addr) => {
                    emu.mem_chunk_addr = addr;
                }
                Command::DumpMemory => {
                    emu.dump_memory();
                }
                Command::ControllerInputs(input) => {
                    emu.bus.controller1.realtime = (input & 0xFF) as u8;
                    emu.bus.controller2.realtime = (input >> 8 & 0xFF) as u8;
                }
            }
        }

        let should_run = !emu.paused || emu.want_step;
        if should_run {
            if let Some(frame) = emu.step_frame() {
                emu.frame_tx.write(frame);

                let elapsed = frame_start_time.elapsed();
                if elapsed < frame_duration {
                    thread::sleep(frame_duration - elapsed);
                }
                frame_start_time = Instant::now();
            }
        } else {
            thread::yield_now();
        }
        if !emu.running {
            break;
        }
    }
    info!("Stopping emulation");
    Ok(())
}

fn save_state(emu: &Emu) -> Result<()> {
    let state = emu.create_state()?;

    let hash = emu
        .bus
        .cart
        .as_ref()
        .context("Cartridge is missing when saving state")?
        .hash
        .clone();

    let cache_dir = get_project_dir(ProjDirKind::Cache)?.join(&hash);
    fs::create_dir_all(&cache_dir)
        .with_context(|| format!("Failed to create cache directory: {}", cache_dir.display()))?;
    let timestamp_millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_millis();
    let path = cache_dir.join(format!("{}.bin", timestamp_millis));
    save_file_compressed(&path, 0, &state)
        .with_context(|| format!("Couldn't save state to {}", path.display()))?;

    info!("Saved state to {}", path.display());
    Ok(())
}

fn load_state(emu: &mut Emu, path: &PathBuf) -> Result<()> {
    let state: EmuState = load_file(path, 0).context("Failed to load file")?;
    emu.load_state(state);
    Ok(())
}

pub enum ProjDirKind {
    Cache,
}

pub fn get_project_dir(dir_kind: ProjDirKind) -> Result<PathBuf> {
    let proj_dirs = ProjectDirs::from("com", "mdmrk", "nessie")
        .context("Could not determine project directories")?;
    Ok(match dir_kind {
        ProjDirKind::Cache => proj_dirs.cache_dir().to_owned(),
    })
}
