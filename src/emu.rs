use std::sync::mpsc;

#[cfg(not(target_arch = "wasm32"))]
use std::fs;

use savefile::prelude::*;

use anyhow::{Context, Result};
use log::{error, info, warn};
use ringbuf::HeapProd;
use ringbuf::traits::Producer;

use crate::{
    bus::Bus,
    cart::Cart,
    cpu::Cpu,
    debug::{DebugSnapshot, MEM_BLOCK_SIZE},
    mapper::MapperEnum,
    platform::FileDataSource,
};
use egui::Color32;

pub enum Command {
    Stop,
    Pause,
    Resume,
    Step,
    MemoryAddress(usize),
    DumpMemory,
    SaveState,
    LoadState(FileDataSource),
    ControllerInputs(u16),
}

pub enum Event {
    Started,
    Stopped,
    Paused,
    Resumed,
    Crashed(String),
}

#[derive(Savefile)]
pub struct EmuState {
    pub cpu: Cpu,
    pub bus: Bus,
    pub mapper: MapperEnum,
    pub cycles_per_sample: f32,
    pub cycles_accumulator: f32,
}

pub struct Emu {
    pub cpu: Cpu,
    pub bus: Bus,
    pub running: bool,
    pub paused: bool,
    pub want_step: bool,
    pub event_tx: mpsc::Sender<Event>,
    pub debug_tx: triple_buffer::Input<DebugSnapshot>,
    pub frame_tx: triple_buffer::Input<Vec<Color32>>,
    pub mem_chunk_addr: usize,
    pub audio_producer: HeapProd<f32>,
    pub cycles_per_sample: f32,
    pub cycles_accumulator: f32,
}

impl Emu {
    pub fn new(
        event_tx: mpsc::Sender<Event>,
        debug_tx: triple_buffer::Input<DebugSnapshot>,
        frame_tx: triple_buffer::Input<Vec<Color32>>,
        enable_logging: bool,
        audio_producer: HeapProd<f32>,
        sample_rate: f32,
    ) -> Self {
        let mut bus = Bus::new();
        bus.apu.set_sample_rate(sample_rate);
        Self {
            cpu: Cpu::new(enable_logging),
            bus,
            running: true,
            paused: false,
            want_step: false,
            event_tx,
            debug_tx,
            frame_tx,
            mem_chunk_addr: 0,
            audio_producer,
            cycles_per_sample: 1789773.0 / sample_rate,
            cycles_accumulator: 0.0,
        }
    }

    pub fn send_event(&self, event: Event) {
        if let Err(e) = self.event_tx.send(event) {
            error!("{e}");
        }
    }

    pub fn load_rom_from_bytes(&mut self, bytes: Vec<u8>) -> Result<()> {
        let cart = Cart::from_bytes(bytes)?;
        self.bus.insert_cartridge(cart);
        info!("Rom loaded from bytes");
        self.bus.ppu.reset();
        self.cpu.reset(&mut self.bus);
        self.send_event(Event::Started);
        Ok(())
    }

    pub fn load_rom(&mut self, rom_path: &str) -> Result<()> {
        let cart = Cart::insert(rom_path)?;
        self.bus.insert_cartridge(cart);
        info!("Rom \"{}\" loaded", rom_path);
        self.bus.ppu.reset();
        self.cpu.reset(&mut self.bus);
        self.send_event(Event::Started);
        Ok(())
    }

    pub fn create_state(&self) -> Result<EmuState> {
        Ok(EmuState {
            cpu: self.cpu.clone(),
            bus: self.bus.clone(),
            mapper: self
                .bus
                .cart
                .as_ref()
                .context("Cartridge is missing when saving state")?
                .mapper
                .clone(),
            cycles_per_sample: self.cycles_per_sample,
            cycles_accumulator: self.cycles_accumulator,
        })
    }

    pub fn load_state(&mut self, state: EmuState) {
        self.cpu = state.cpu;
        self.bus.mem = state.bus.mem;
        self.bus.apu = state.bus.apu;
        self.bus.ppu = state.bus.ppu;
        self.bus.ppu.screen = vec![Color32::BLACK; 256 * 240];
        self.bus.controller1 = state.bus.controller1;
        self.bus.controller2 = state.bus.controller2;
        self.bus.open_bus = state.bus.open_bus;
        if let Some(cart) = self.bus.cart.as_mut() {
            cart.mapper = state.mapper;
        }

        self.cycles_per_sample = state.cycles_per_sample;
        self.cycles_accumulator = state.cycles_accumulator;
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

    pub fn step_frame(&mut self) -> Option<Vec<Color32>> {
        let mut frame_out = None;
        if !self.paused || self.want_step {
            loop {
                let cycles_before = self.cpu.cycles;
                if let Err(e) = self.cpu.step(&mut self.bus) {
                    warn!("{e}. Emulator will be paused");
                    self.pause();
                    break;
                }

                if self.bus.dma_stall > 0 {
                    let stall = self.bus.dma_stall;
                    self.bus.dma_stall = 0;
                    if let Some(cart) = self.bus.cart.as_mut() {
                        self.bus.ppu.step(&mut cart.mapper, stall);
                    }
                    self.cpu.cycles += stall;
                }

                let cycles_delta = self.cpu.cycles - cycles_before;

                for _ in 0..cycles_delta {
                    self.bus.tick_apu();
                    self.cycles_accumulator += 1.0;

                    if self.cycles_accumulator >= self.cycles_per_sample {
                        let sample = self.bus.apu.output();
                        let _ = self.audio_producer.try_push(sample);
                        self.cycles_accumulator -= self.cycles_per_sample;
                    }
                }

                self.cpu.irq_pending = self.bus.apu.irq_occurred();

                if self.bus.ppu.frame_ready {
                    self.bus.ppu.frame_ready = false;
                    frame_out = Some(self.bus.ppu.screen.clone());

                    let memory_slice = self
                        .bus
                        .read_only_range(self.mem_chunk_addr as u16, MEM_BLOCK_SIZE as u16);
                    let stack_slice = self.bus.read_only_range(0x100, 0x100);

                    let snapshot = DebugSnapshot::new(
                        &self.cpu,
                        &self.bus.ppu,
                        &self.bus.apu,
                        self.bus.cart.as_ref(),
                        &memory_slice,
                        &stack_slice,
                    );
                    self.debug_tx.write(snapshot);

                    break;
                }
            }
            self.want_step = false;
        }
        frame_out
    }

    pub fn dump_memory(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut mem: [u8; 0x10000] = [0; 0x10000];

            for (i, n) in mem.iter_mut().enumerate() {
                *n = self.bus.read_only(i as u16);
            }
            let mut path = std::env::current_exe().unwrap();
            path.set_file_name("dump.txt");
            info!("Memory dumped to {:?}", path);
            fs::write(path, mem).expect("Cannot write into memory");
        }
    }
}
