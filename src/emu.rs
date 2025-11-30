use std::{
    fs,
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
};
use egui::Color32;

pub enum Command {
    Stop,
    Pause,
    Resume,
    Step,
    MemoryAddress(usize),
    DumpMemory,
    ControllerInputs(u16),
}

pub enum Event {
    Stopped,
    Paused,
    Resumed,
    Crashed(String),
    FrameReady(Vec<Color32>),
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
                let cycles_before = self.cpu.cycle_count;
                if let Err(e) = self.cpu.step(&mut self.bus) {
                    warn!("{e}. Emulator will be paused");
                    self.paused = true;
                    break;
                }

                let cycles_delta = self.cpu.cycle_count - cycles_before;

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
                    let cart_header = self.bus.cart.as_ref().map(|c| &c.header);

                    let snapshot = DebugSnapshot::new(
                        &self.cpu,
                        &self.bus.ppu,
                        &self.bus.apu,
                        cart_header,
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
