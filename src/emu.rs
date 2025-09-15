use std::sync::{Arc, mpsc};

use crate::{cpu, debug::DebugState};

pub enum Command {
    Start,
    Stop,
}

pub enum Event {
    Started,
    Stopped,
}

pub struct Emu {
    pub cpu: cpu::Cpu,
    pub running: bool,
}

impl Emu {
    pub fn new() -> Self {
        Self {
            cpu: cpu::Cpu::new(),
            running: false,
        }
    }

    pub fn start(&self) {}

    pub fn stop(&self) {}
}

pub fn emu_thread(
    command_rx: mpsc::Receiver<Command>,
    event_tx: mpsc::Sender<Event>,
    debug_state: Arc<DebugState>,
) {
    let mut emu = Emu::new();

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
