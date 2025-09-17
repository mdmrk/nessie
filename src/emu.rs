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
            self.cart = Some(cart);
            info!("Rom '{}' loaded", rom_path);
        }
    }

    pub fn start(&self) {}

    pub fn stop(&self) {}
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
                    emu.running = true;
                }
                Command::Stop => {
                    emu.stop();
                    emu.running = false;
                }
            }
        }

        emu.cpu.step();
        debug_state.update(&emu);
    }
}
