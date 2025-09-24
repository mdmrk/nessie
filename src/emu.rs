use std::{
    io::Read,
    sync::{Arc, mpsc},
};

use log::info;

use crate::{args::Args, bus::Bus, cart::Cart, cpu::Cpu, debug::DebugState};

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
    pub bus: Bus,
    pub cart: Option<Cart>,
    pub running: bool,
    pub paused: bool,
    pub want_step: bool,
}

impl Emu {
    pub fn new() -> Self {
        Self {
            cpu: Cpu::new(),
            bus: Bus::new(),
            cart: None,
            running: true,
            paused: false,
            want_step: false,
        }
    }

    pub fn load_rom(&mut self, rom_path: &String) {
        if let Some(cart) = Cart::insert(rom_path) {
            if cart.header.get_mapper() == 0 {
                let size = cart.header.prg_rom_size as usize;
                let data = cart.prg_data.get(0..size * 1024).unwrap();
                self.bus.write(0x8000, data);
                self.bus.write(0xC000, data); // mirror (16KB)
                self.cpu.pc = 0xC000; // FIXME: TEST Only
            } else if cart.header.get_mapper() == 1 {
                let size = cart.header.prg_rom_size as usize;
                let data = cart.prg_data.get(0..size * 1024).unwrap();
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

        if emu.running {
            if emu.paused {
                if emu.want_step {
                    emu.cpu.step(&emu.bus);
                    emu.want_step = false;
                }
            } else {
                emu.cpu.step(&emu.bus);
            }
        }
        debug_state.update(&emu);
    }
}
