use egui::Color32;
use log::error;
use rfd::AsyncFileDialog;
use ringbuf::{HeapRb, traits::Split};
use std::sync::mpsc;

use crate::audio::Audio;
use crate::debug::DebugSnapshot;
use crate::emu::{Command, Emu, Event};

pub struct PlatformRunner {
    pub emu: Option<Emu>,
    pub audio: Option<Audio>,
    pub running: bool,
    pub paused: bool,
    pub pending_events: Vec<Event>,
    pub rom_loader_rx: Option<mpsc::Receiver<(Vec<u8>, crate::args::Args)>>,
}

impl PlatformRunner {
    pub fn new() -> Self {
        Self {
            emu: None,
            audio: None,
            running: false,
            paused: false,
            pending_events: Vec::new(),
            rom_loader_rx: None,
        }
    }

    pub fn start(&mut self, rom: RomSource, _args: crate::args::Args) {
        let rb = HeapRb::<f32>::new(4096);
        let (producer, consumer) = rb.split();

        let (audio_handle, sample_rate) = match Audio::new(consumer) {
            Ok(audio) => {
                let rate = audio.sample_rate;
                (Some(audio), rate)
            }
            Err(e) => {
                error!("Failed to initialize audio: {}", e);
                (None, 44100.0)
            }
        };
        self.audio = audio_handle;

        let (tx, _rx) = mpsc::channel();
        let (debug_tx, _debug_rx) = mpsc::channel();

        let mut emu = Emu::new(tx, debug_tx, false, producer, sample_rate);

        match rom {
            RomSource::Bytes(bytes) => {
                if emu.load_rom_from_bytes(bytes).is_err() {
                    error!("Failed to load ROM from bytes");
                    return;
                }
            }
            _ => {
                error!("Wasm only supports loading from bytes");
                return;
            }
        }

        self.emu = Some(emu);
        self.running = true;
        self.paused = false;
    }

    pub fn stop(&mut self) {
        self.emu = None;
        self.running = false;
        self.paused = false;
    }

    pub fn pause(&mut self) {
        if let Some(emu) = &mut self.emu {
            emu.paused = true;
            self.paused = true;
        }
    }

    pub fn resume(&mut self) {
        if let Some(emu) = &mut self.emu {
            emu.paused = false;
            self.paused = false;
        }
    }

    pub fn step(&mut self) {
        if let Some(emu) = &mut self.emu {
            emu.want_step = true;
        }
    }

    pub fn send_command(&mut self, command: Command) {
        if let Some(emu) = &mut self.emu {
            match command {
                Command::Stop => {
                    self.stop();
                }
                Command::Pause => {
                    self.pause();
                }
                Command::Resume => {
                    self.resume();
                }
                Command::Step => {
                    self.step();
                }
                Command::ControllerInputs(input) => {
                    emu.bus.controller1.realtime = (input & 0xFF) as u8;
                    emu.bus.controller2.realtime = (input >> 8 & 0xFF) as u8;
                }
                Command::MemoryAddress(addr) => {
                    emu.mem_chunk_addr = addr;
                }
                Command::DumpMemory => {
                    emu.dump_memory();
                }
            }
        }
    }

    pub fn handle_events(&mut self, _ctx: &egui::Context) -> Vec<Event> {
        let loaded_rom = if let Some(rx) = &self.rom_loader_rx {
            rx.try_recv().ok()
        } else {
            None
        };

        if let Some((data, args)) = loaded_rom {
            self.start(RomSource::Bytes(data), args);
            self.rom_loader_rx = None;
        }

        let mut events = std::mem::take(&mut self.pending_events);

        if let Some(emu) = &mut self.emu {
            if let Some(frame) = emu.step_frame() {
                events.push(Event::FrameReady(frame));
            }
        }

        events
    }

    pub fn get_debug_snapshot(&self) -> Option<DebugSnapshot> {
        None
    }

    pub fn pick_rom(&mut self, args: crate::args::Args) {
        let (tx, rx) = mpsc::channel();
        self.rom_loader_rx = Some(rx);

        let task = async move {
            if let Some(file) = AsyncFileDialog::new()
                .add_filter("NES rom", &["nes"])
                .pick_file()
                .await
            {
                let data = file.read().await;
                let _ = tx.send((data, args));
            }
        };
        wasm_bindgen_futures::spawn_local(task);
    }

    pub fn pick_state_file(&self) {
        // TODO
    }
}

impl Default for PlatformRunner {
    fn default() -> Self {
        Self::new()
    }
}
