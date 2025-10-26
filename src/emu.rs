use std::sync::{Arc, mpsc};

use log::{error, info, warn};

use crate::{
    args::Args,
    bus::Bus,
    cart::Cart,
    cpu::Cpu,
    debug::{DebugLog, DebugState},
    ppu::Ppu,
};

pub enum Command {
    Stop,
    Pause,
    Resume,
    Step,
    MemoryAddress(usize),
}

pub enum Event {
    Stopped,
    Paused,
    Resumed,
    Crashed,
}

pub struct Emu {
    pub cpu: Cpu,
    pub ppu: Ppu,
    pub bus: Bus,
    pub running: bool,
    pub paused: bool,
    pub want_step: bool,
    pub debug_log: Option<DebugLog>,
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
            debug_log: None,
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

    pub fn step(&mut self) {
        self.want_step = true;
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
    if let Some(logfile) = &args.logfile {
        emu.debug_log = Some(DebugLog::new(logfile));
    }
    if args.pause {
        emu.pause();
    }

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
                    emu.step();
                }
                Command::MemoryAddress(addr) => {
                    emu.mem_chunk_addr = addr;
                }
            }
        }

        let should_run = !emu.paused || emu.want_step;
        if should_run {
            if let Err(e) = emu.cpu.step(&mut emu.bus, &mut emu.ppu, &mut emu.debug_log) {
                warn!("{e}");
                emu.pause();
            }
            emu.want_step = false;
        }
        if !emu.running {
            break;
        }
        debug_state.update(&mut emu);
    }
    info!("Stopping emulation");
}
