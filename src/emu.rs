use std::sync::mpsc;

use crate::cpu;

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
}

pub fn emu_thread(command_rx: mpsc::Receiver<Command>, event_tx: mpsc::Sender<Event>) {
    let mut emu = Emu::new();

    loop {
        while let Ok(command) = command_rx.try_recv() {
            match command {
                Command::Start => {
                    emu.running = true;
                }
                Command::Stop => {
                    emu.running = false;
                }
            }
        }

        emu.cpu.step();
    }
}
