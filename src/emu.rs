use std::sync::{Arc, mpsc};

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
            running: false,
        }
    }

    pub fn load_rom(&mut self, rom_path: &String) {
        if let Some(cart) = Cart::insert(rom_path) {
            let data = cart.prg_data.get(0..16 * 1024).unwrap();
            self.bus.write(0x8000, data);
            let data2 = cart.prg_data.get(16 * 1024 * 15..16 * 1024 * 16).unwrap();
            self.bus.write(0xBFFF, data2);
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
            emu.cpu.step(&emu.bus);
        }
        debug_state.update(&emu);
    }
}
