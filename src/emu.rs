#[cfg(not(target_arch = "wasm32"))]
use savefile::{prelude::*, save_file_compressed};
#[cfg(not(target_arch = "wasm32"))]
use std::fs;

use std::{
    path::PathBuf,
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

use log::{error, info, warn};
use ringbuf::HeapProd;
use ringbuf::traits::Producer;

use crate::{
    args::Args,
    bus::Bus,
    cart::Cart,
    cpu::Cpu,
    debug::{DebugSnapshot, MEM_BLOCK_SIZE},
    mapper::MapperEnum,
};
use egui::Color32;

pub enum Command {
    Stop,
    Pause,
    Resume,
    Step,
    MemoryAddress(usize),
    DumpMemory,
    SaveState,
    LoadState(PathBuf),
    ControllerInputs(u16),
}

pub enum Event {
    Stopped,
    Paused,
    Resumed,
    Crashed(String),
    FrameReady(Vec<Color32>),
}

#[cfg_attr(not(target_arch = "wasm32"), derive(Savefile))]
struct EmuState {
    pub cpu: Cpu,
    pub bus: Bus,
    pub mapper: MapperEnum,
    pub cycles_per_sample: f32,
    pub cycles_accumulator: f32,
    pub sample_sum: f32,
    pub sample_count: f32,
}

pub struct Emu {
    pub cpu: Cpu,
    pub bus: Bus,
    pub running: bool,
    pub paused: bool,
    pub want_step: bool,
    pub event_tx: mpsc::Sender<Event>,
    pub debug_tx: mpsc::Sender<DebugSnapshot>,
    pub mem_chunk_addr: usize,
    pub audio_producer: HeapProd<f32>,
    pub cycles_per_sample: f32,
    pub cycles_accumulator: f32,
    pub sample_sum: f32,
    pub sample_count: f32,
}

impl Emu {
    pub fn new(
        event_tx: mpsc::Sender<Event>,
        debug_tx: mpsc::Sender<DebugSnapshot>,
        enable_logging: bool,
        audio_producer: HeapProd<f32>,
        sample_rate: f32,
    ) -> Self {
        Self {
            cpu: Cpu::new(enable_logging),
            bus: Bus::new(),
            running: true,
            paused: false,
            want_step: false,
            event_tx,
            debug_tx,
            mem_chunk_addr: 0,
            audio_producer,
            cycles_per_sample: 1789773.0 / sample_rate,
            cycles_accumulator: 0.0,
            sample_sum: 0.0,
            sample_count: 0.0,
        }
    }

    pub fn send_event(&self, event: Event) {
        if let Err(e) = self.event_tx.send(event) {
            error!("{e}");
        }
    }

    pub fn load_rom_from_bytes(&mut self, bytes: Vec<u8>) {
        if let Some(cart) = Cart::from_bytes(bytes) {
            self.bus.insert_cartridge(cart);
            info!("Rom loaded from bytes");
            self.bus.ppu.reset();
            self.cpu.reset(&mut self.bus);
        }
    }

    pub fn load_rom(&mut self, rom_path: &str) {
        if let Some(cart) = Cart::insert(rom_path) {
            self.bus.insert_cartridge(cart);
            info!("Rom \"{}\" loaded", rom_path);
            self.bus.ppu.reset();
            self.cpu.reset(&mut self.bus);
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn save_state(&self) -> anyhow::Result<()> {
        use anyhow::Context;

        let state = EmuState {
            cpu: self.cpu.clone(),
            bus: self.bus.clone(),
            mapper: self
                .bus
                .cart
                .as_ref()
                .context("Cartridge is missing when saving state")?
                .mapper
                .clone(),
            cycles_per_sample: self.cycles_per_sample,
            cycles_accumulator: self.cycles_accumulator,
            sample_sum: self.sample_sum,
            sample_count: self.sample_count,
        };
        let hash = self
            .bus
            .cart
            .as_ref()
            .context("Cartridge is missing when saving state")?
            .hash
            .clone();

        let cache_dir = get_project_dir(ProjDirKind::Cache)?.join(&hash);
        std::fs::create_dir_all(&cache_dir).with_context(|| {
            format!("Failed to create cache directory: {}", cache_dir.display())
        })?;
        let timestamp_millis = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_millis();
        let path = cache_dir.join(format!("{}.bin", timestamp_millis));
        save_file_compressed(&path, 0, &state)
            .with_context(|| format!("Couldn't save state to {}", path.display()))?;

        info!("Saved state to {}", path.display());
        Ok(())
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn load_state(&mut self, path: &PathBuf) -> anyhow::Result<()> {
        let file = load_emu_state(path)?;

        self.cpu = file.cpu;

        self.bus.mem = file.bus.mem;
        self.bus.apu = file.bus.apu;
        self.bus.ppu = file.bus.ppu;
        self.bus.ppu.screen = vec![Color32::BLACK; 256 * 240];
        self.bus.controller1 = file.bus.controller1;
        self.bus.controller2 = file.bus.controller2;
        self.bus.open_bus = file.bus.open_bus;
        self.bus.cart.as_mut().unwrap().mapper = file.mapper;

        self.cycles_per_sample = file.cycles_per_sample;
        self.cycles_accumulator = file.cycles_accumulator;
        self.sample_sum = file.sample_sum;
        self.sample_count = file.sample_count;

        Ok(())
    }

    pub fn stop(&mut self) {
        self.running = false;
        self.send_event(Event::Stopped);
    }

    pub fn pause(&mut self) {
        self.paused = true;
        self.send_event(Event::Paused);
    }

    pub fn resume(&mut self) {
        self.paused = false;
        self.send_event(Event::Resumed);
    }

    pub fn step_frame(&mut self) -> Option<Vec<Color32>> {
        let mut frame_out = None;
        if !self.paused || self.want_step {
            loop {
                let cycles_before = self.cpu.cycles;
                if let Err(e) = self.cpu.step(&mut self.bus) {
                    warn!("{e}. Emulator will be paused");
                    self.paused = true;
                    break;
                }

                let cycles_delta = self.cpu.cycles - cycles_before;

                for _ in 0..cycles_delta {
                    self.bus.tick_apu();
                    self.sample_sum += self.bus.apu.output();
                    self.sample_count += 1.0;
                    self.cycles_accumulator += 1.0;

                    if self.cycles_accumulator >= self.cycles_per_sample {
                        let sample = if self.sample_count > 0.0 {
                            self.sample_sum / self.sample_count
                        } else {
                            0.0
                        };
                        let _ = self.audio_producer.try_push(sample);

                        self.cycles_accumulator -= self.cycles_per_sample;
                        self.sample_sum = 0.0;
                        self.sample_count = 0.0;
                    }
                }

                if self.bus.ppu.frame_ready {
                    self.bus.ppu.frame_ready = false;
                    frame_out = Some(self.bus.ppu.screen.clone());

                    let memory_slice = self
                        .bus
                        .read_only_range(self.mem_chunk_addr as u16, MEM_BLOCK_SIZE as u16);
                    let stack_slice = self.bus.read_only_range(0x100, 0x100);

                    let snapshot = DebugSnapshot::new(
                        &self.cpu,
                        &self.bus.ppu,
                        &self.bus.apu,
                        self.bus.cart.as_ref(),
                        &memory_slice,
                        &stack_slice,
                    );
                    let _ = self.debug_tx.send(snapshot);

                    break;
                }
            }
            self.want_step = false;
        }
        frame_out
    }

    fn dump_memory(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut mem: [u8; 0x10000] = [0; 0x10000];

            for (i, n) in mem.iter_mut().enumerate() {
                *n = self.bus.read_only(i as u16);
            }
            let mut path = std::env::current_exe().unwrap();
            path.set_file_name("dump.txt");
            info!("Memory dumped to {:?}", path);
            fs::write(path, mem).expect("Cannot write into memory");
        }
    }
}

pub fn emu_thread(
    command_rx: mpsc::Receiver<Command>,
    event_tx: mpsc::Sender<Event>,
    debug_tx: mpsc::Sender<DebugSnapshot>,
    args: &Args,
    rom: &str,
    audio_producer: HeapProd<f32>,
    sample_rate: f32,
) {
    let mut emu = Emu::new(event_tx, debug_tx, args.log, audio_producer, sample_rate);

    emu.load_rom(rom);
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
                Command::SaveState => {
                    emu.save_state()
                        .unwrap_or_else(|e| error!("Failed to save state: {e}"));
                }
                Command::LoadState(path) => {
                    emu.load_state(&path)
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
                emu.send_event(Event::FrameReady(frame));

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
}

#[cfg(not(target_arch = "wasm32"))]
fn load_emu_state(path: &PathBuf) -> anyhow::Result<EmuState> {
    use anyhow::Context;

    load_file(path, 0).context("Failed to load file")
}

#[cfg(not(target_arch = "wasm32"))]
pub enum ProjDirKind {
    Cache,
}

#[cfg(not(target_arch = "wasm32"))]
pub fn get_project_dir(dir_kind: ProjDirKind) -> anyhow::Result<PathBuf> {
    use anyhow::Context;
    use directories::ProjectDirs;

    let proj_dirs = ProjectDirs::from("com", "mdmrk", "nessie")
        .context("Could not determine project directories")?;
    Ok(match dir_kind {
        ProjDirKind::Cache => proj_dirs.cache_dir().to_owned(),
    })
}
