use std::{
    fs,
    sync::{Arc, mpsc},
    thread,
    time::{Duration, Instant},
};

use log::{error, info, warn};

use crate::{args::Args, bus::Bus, cart::Cart, cpu::Cpu, debug::DebugState, ppu::Ppu};
use egui::Color32;

pub enum Command {
    Stop,
    Pause,
    Resume,
    Step,
    MemoryAddress(usize),
    DumpMemory,
    Update,
    ControllerInputs(u16),
}

pub enum Event {
    Stopped,
    Paused,
    Resumed,
    Crashed,
    FrameReady(Vec<Color32>),
}

pub struct Emu {
    pub cpu: Cpu,
    pub ppu: Ppu,
    pub bus: Bus,
    pub running: bool,
    pub paused: bool,
    pub want_step: bool,
    pub event_tx: mpsc::Sender<Event>,
    pub mem_chunk_addr: usize,
}

impl Emu {
    pub fn new(event_tx: mpsc::Sender<Event>) -> Self {
        Self {
            cpu: Cpu::new(),
            ppu: Ppu::new(),
            bus: Bus::new(),
            running: true,
            paused: false,
            want_step: false,
            event_tx,
            mem_chunk_addr: 0,
        }
    }

    pub fn send_event(&self, event: Event) {
        if let Err(e) = self.event_tx.send(event) {
            error!("{e}");
        }
    }

    pub fn load_rom(&mut self, rom_path: &str) {
        if let Some(cart) = Cart::insert(rom_path) {
            self.bus.insert_cartridge(cart);
            info!("Rom \"{}\" loaded", rom_path);
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

    fn dump_memory(&mut self) {
        let mut mem: [u8; 0x10000] = [0; 0x10000];

        for (i, n) in mem.iter_mut().enumerate() {
            *n = self.bus.read_byte(i);
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
    debug_state: Arc<DebugState>,
    args: &Args,
    rom: &str,
) {
    let mut emu = Emu::new(event_tx);

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
                Command::Update => {
                    debug_state.update(&mut emu);
                }
                Command::ControllerInputs(input) => {
                    emu.bus.controller1.realtime = (input & 0xFF) as u8;
                    emu.bus.controller2.realtime = (input >> 8 & 0xFF) as u8;
                }
            }
        }

        let should_run = !emu.paused || emu.want_step;
        if should_run {
            if let Err(e) = emu.cpu.step(&mut emu.bus, &mut emu.ppu, args) {
                warn!("{e}. Emulator will be paused");
                emu.pause();
            }
            if emu.ppu.frame_ready {
                emu.ppu.frame_ready = false;
                let frame_arc = emu.ppu.screen.clone();
                emu.send_event(Event::FrameReady(frame_arc));

                let elapsed = frame_start_time.elapsed();
                if elapsed < frame_duration {
                    thread::sleep(frame_duration - elapsed);
                }
                frame_start_time = Instant::now();
            }
            emu.want_step = false;
        } else {
            thread::yield_now();
        }
        if !emu.running {
            break;
        }
    }
    info!("Stopping emulation");
}
