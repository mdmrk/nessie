use std::{
    io::Read,
    sync::{Arc, mpsc},
};

use log::info;

use crate::{args::Args, bus::Bus, cart::Cart, cpu::Cpu, debug::DebugState};

pub enum Command {
    Start,
    Stop,
}

pub enum Event {
    Started,
    Stopped,
}

pub struct Emu {
    pub cpu: Cpu,
    pub bus: Bus,
    pub running: bool,
    pub cart: Option<Cart>,
}

impl Emu {
    pub fn new() -> Self {
        Self {
            cpu: Cpu::new(),
            bus: Bus::new(),
            cart: None,
            running: true,
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

    pub fn start(&mut self) {
        self.running = true;
    }

    pub fn stop(&mut self) {
        self.running = false;
    }
}

pub fn emu_thread(
    command_rx: mpsc::Receiver<Command>,
    _event_tx: mpsc::Sender<Event>,
    debug_state: Arc<DebugState>,
    args: &Args,
) {
    let mut emu = Emu::new();

    if let Some(rom) = &args.rom {
        emu.load_rom(rom);
    }

    loop {
        while let Ok(command) = command_rx.try_recv() {
            match command {
                Command::Start => {
                    emu.start();
                }
                Command::Stop => {
                    emu.stop();
                }
            }
        }

        if emu.running {
            println!(
                "{:04X}  4C F5 C5  JMP $C5F5                       A:00 X:00 Y:00 P:24 SP:FD PPU:  0, 21 CYC:7",
                emu.cpu.pc
            );
            emu.cpu.step(&emu.bus);
        }
        debug_state.update(&emu);
    }
}
