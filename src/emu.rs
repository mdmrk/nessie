use std::sync::{Arc, mpsc};

use log::info;

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
}

pub enum Event {
    Stopped,
    Paused,
    Resumed,
}

pub struct Emu {
    pub cpu: Cpu,
    pub ppu: Ppu,
    pub bus: Bus,
    pub running: bool,
    pub paused: bool,
    pub want_step: bool,
    pub debug_log: Option<DebugLog>,
}

impl Default for Emu {
    fn default() -> Self {
        Self::new()
    }
}

impl Emu {
    pub fn new() -> Self {
        Self {
            cpu: Cpu::new(),
            ppu: Ppu::new(),
            bus: Bus::new(),
            running: true,
            paused: false,
            want_step: false,
            debug_log: None,
        }
    }

    pub fn load_rom(&mut self, rom_path: &String) {
        if let Some(cart) = Cart::insert(rom_path) {
            self.bus.insert_cartridge(cart);
            info!("Rom \"{}\" loaded", rom_path);
            self.cpu.reset(&mut self.bus);
        }
    }

    pub fn stop(&mut self) {
        self.running = false;
    }

    pub fn pause(&mut self) {
        self.paused = true;
    }

    pub fn resume(&mut self) {
        self.paused = false;
    }

    pub fn step(&mut self) {
        self.want_step = true;
    }
}

pub fn emu_thread(command_rx: mpsc::Receiver<Command>, debug_state: Arc<DebugState>, args: &Args) {
    let mut emu = Emu::new();

    if let Some(rom) = &args.rom {
        emu.load_rom(rom);
    }
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
            }
        }

        let should_run = !emu.paused || emu.want_step;
        if should_run {
            let ok = emu.cpu.step(&mut emu.bus, &mut emu.ppu, &mut emu.debug_log);
            if !ok {
                emu.pause();
            }
            emu.want_step = false;
        }
        debug_state.update(&mut emu);
    }
}
