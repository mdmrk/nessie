use std::{
    sync::{Arc, mpsc},
    thread,
    time::{Duration, Instant},
};

use log::{debug, info, trace};

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
    pub cart: Option<Cart>,
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
            cart: None,
            running: true,
            paused: false,
            want_step: false,
            debug_log: None,
        }
    }

    pub fn load_rom(&mut self, rom_path: &String) {
        if let Some(cart) = Cart::insert(rom_path) {
            if cart.header.get_mapper() == 0 {
                let size = cart.header.prg_rom_size as usize;
                if size == 1 {
                    let data = cart.prg_data.get(0..16 * 1024).unwrap();
                    self.bus.write(0x8000, data);
                    self.bus.write(0xC000, data); // mirror (16KB)
                    self.cpu.pc = 0xC000; // FIXME: TEST Only
                } else if size == 2 {
                    let data = cart.prg_data.get(0..16 * 1024).unwrap();
                    self.bus.write(0x8000, data);
                    let data2 = cart.prg_data.get(16 * 1024..2 * 16 * 1024).unwrap();
                    self.bus.write(0xC000, data2);
                    let reset_vector = self.bus.read(0xFFFC, 2);
                    self.cpu.pc = u16::from_le_bytes([reset_vector[0], reset_vector[1]]); // FIXME: TEST Only
                }
            } else if cart.header.get_mapper() == 1 {
                let size = cart.header.prg_rom_size as usize;
                let data = cart.prg_data.get(0..16 * 1024).unwrap();
                self.bus.write(0x8000, data);
                let offset = (size - 1) * 16 * 1024;
                let data2 = cart.prg_data.get(offset..offset + 16 * 1024).unwrap();
                self.bus.write(0xC000, data2);
                self.cpu.pc =
                    (self.bus.read_byte(0xfffc) as u16) << 4 | self.bus.read_byte(0xfffd) as u16;
            }
            self.cart = Some(cart);
            info!("Rom \"{}\" loaded", rom_path);
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

    let fps_limit = 60;
    let frame_interval = Duration::from_secs(1) / fps_limit;
    let mut last_time = Instant::now();
    debug!("FPS limit: {:} FPS ({:?})", fps_limit, frame_interval);

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

        let elapsed = last_time.elapsed();
        if elapsed < frame_interval {
            thread::sleep(frame_interval - elapsed);
        }
        last_time = Instant::now();
    }
}
