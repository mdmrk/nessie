use modular_bitfield::prelude::*;

static LENGTH_TABLE: [u8; 32] = [
    10, 254, 20, 2, 40, 4, 80, 6, 160, 8, 60, 10, 14, 12, 26, 14, 12, 16, 24, 18, 48, 20, 96, 22,
    192, 24, 72, 26, 16, 28, 32, 30,
];

static DUTY_TABLE: [[u8; 8]; 4] = [
    [0, 1, 0, 0, 0, 0, 0, 0],
    [0, 1, 1, 0, 0, 0, 0, 0],
    [0, 1, 1, 1, 1, 0, 0, 0],
    [1, 0, 0, 1, 1, 1, 1, 1],
];

static NOISE_PERIOD_TABLE: [u16; 16] = [
    4, 8, 16, 32, 64, 96, 128, 160, 202, 254, 380, 508, 762, 1016, 2034, 4068,
];

static DMC_RATE_TABLE: [u16; 16] = [
    428, 380, 340, 320, 286, 254, 226, 214, 190, 160, 142, 128, 106, 84, 72, 54,
];

#[bitfield(bytes = 1)]
#[derive(Debug, Clone, Default, Copy)]
pub struct ApuStatus {
    pub enable_dmc: bool,
    pub enable_noise: bool,
    pub enable_triangle: bool,
    pub enable_pulse2: bool,
    pub enable_pulse1: bool,
    pub dmc_interrupt: bool,
    pub frame_interrupt: bool,
    pub dmc_active: bool,
}

#[derive(Debug, Default, Clone)]
pub struct Envelope {
    pub start: bool,
    pub disable: bool,
    pub divider_period: u8,
    pub decay_count: u8,
    pub divider_count: u8,
    pub loop_mode: bool,
    pub constant_volume: bool,
}

impl Envelope {
    fn step(&mut self) {
        if !self.start {
            if self.divider_count == 0 {
                self.divider_count = self.divider_period;
                if self.decay_count == 0 {
                    if self.loop_mode {
                        self.decay_count = 15;
                    }
                } else {
                    self.decay_count -= 1;
                }
            } else {
                self.divider_count -= 1;
            }
        } else {
            self.start = false;
            self.decay_count = 15;
            self.divider_count = self.divider_period;
        }
    }

    fn output(&self) -> u8 {
        if self.constant_volume {
            self.divider_period
        } else {
            self.decay_count
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct Sweep {
    pub enabled: bool,
    pub period: u8,
    pub negate: bool,
    pub shift: u8,
    pub reload: bool,
    pub divider: u8,
    pub mute: bool,
}

impl Sweep {
    fn step(&mut self, timer: &mut u16, ones_complement: bool) {
        let change_amount = *timer >> self.shift;
        let mut target_period = *timer;

        if self.negate {
            target_period = target_period.wrapping_sub(change_amount);
            if ones_complement {
                target_period = target_period.wrapping_sub(1);
            }
        } else {
            target_period = target_period.wrapping_add(change_amount);
        }

        self.mute = *timer < 8 || target_period > 0x7FF;

        if self.divider == 0 && self.enabled && !self.mute && self.shift > 0 {
            *timer = target_period;
        }

        if self.divider == 0 || self.reload {
            self.divider = self.period;
            self.reload = false;
        } else {
            self.divider -= 1;
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct Pulse {
    pub enabled: bool,
    pub channel_idx: u8,
    pub length_value: u8,
    pub length_halt: bool,
    pub timer: u16,
    pub timer_period: u16,
    pub duty_mode: u8,
    pub duty_clock: u8,
    pub envelope: Envelope,
    pub sweep: Sweep,
}

impl Pulse {
    fn new(idx: u8) -> Self {
        Self {
            channel_idx: idx,
            ..Default::default()
        }
    }

    fn write_ctrl(&mut self, value: u8) {
        self.duty_mode = (value >> 6) & 0x03;
        self.length_halt = (value & 0x20) != 0;
        self.envelope.loop_mode = (value & 0x20) != 0;
        self.envelope.constant_volume = (value & 0x10) != 0;
        self.envelope.divider_period = value & 0x0F;
    }

    fn write_sweep(&mut self, value: u8) {
        self.sweep.enabled = (value & 0x80) != 0;
        self.sweep.period = (value >> 4) & 0x07;
        self.sweep.negate = (value & 0x08) != 0;
        self.sweep.shift = value & 0x07;
        self.sweep.reload = true;
    }

    fn write_timer_low(&mut self, value: u8) {
        self.timer_period = (self.timer_period & 0xFF00) | value as u16;
    }

    fn write_timer_high(&mut self, value: u8) {
        self.timer_period = (self.timer_period & 0x00FF) | ((value as u16 & 0x07) << 8);
        if self.enabled {
            self.length_value = LENGTH_TABLE[(value >> 3) as usize];
        }
        self.timer = self.timer_period;
        self.duty_clock = 0;
        self.envelope.start = true;
    }

    fn step_timer(&mut self) {
        if self.timer == 0 {
            self.timer = self.timer_period;
            self.duty_clock = (self.duty_clock + 1) % 8;
        } else {
            self.timer -= 1;
        }
    }

    fn step_length(&mut self) {
        if !self.length_halt && self.length_value > 0 {
            self.length_value -= 1;
        }
    }

    fn step_envelope(&mut self) {
        self.envelope.step();
    }

    fn step_sweep(&mut self) {
        self.sweep
            .step(&mut self.timer_period, self.channel_idx == 0);
    }

    fn output(&self) -> u8 {
        if !self.enabled || self.length_value == 0 || self.sweep.mute || self.timer_period < 8 {
            return 0;
        }
        if DUTY_TABLE[self.duty_mode as usize][self.duty_clock as usize] != 0 {
            self.envelope.output()
        } else {
            0
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct Triangle {
    pub enabled: bool,
    pub length_value: u8,
    pub length_halt: bool,
    pub linear_counter: u8,
    pub linear_reload: bool,
    pub linear_period: u8,
    pub timer: u16,
    pub timer_period: u16,
    pub duty_clock: u8,
}

impl Triangle {
    fn write_linear(&mut self, value: u8) {
        self.length_halt = (value & 0x80) != 0;
        self.linear_period = value & 0x7F;
    }

    fn write_timer_low(&mut self, value: u8) {
        self.timer_period = (self.timer_period & 0xFF00) | value as u16;
    }

    fn write_timer_high(&mut self, value: u8) {
        self.timer_period = (self.timer_period & 0x00FF) | ((value as u16 & 0x07) << 8);
        if self.enabled {
            self.length_value = LENGTH_TABLE[(value >> 3) as usize];
        }
        self.linear_reload = true;
    }

    fn step_timer(&mut self) {
        if self.timer == 0 {
            self.timer = self.timer_period;
            if self.length_value > 0 && self.linear_counter > 0 {
                self.duty_clock = (self.duty_clock + 1) % 32;
            }
        } else {
            self.timer -= 1;
        }
    }

    fn step_length(&mut self) {
        if !self.length_halt && self.length_value > 0 {
            self.length_value -= 1;
        }
    }

    fn step_linear(&mut self) {
        if self.linear_reload {
            self.linear_counter = self.linear_period;
        } else if self.linear_counter > 0 {
            self.linear_counter -= 1;
        }
        if !self.length_halt {
            self.linear_reload = false;
        }
    }

    fn output(&self) -> u8 {
        if !self.enabled
            || self.length_value == 0
            || self.linear_counter == 0
            || self.timer_period < 2
        {
            0
        } else {
            let val = self.duty_clock;
            if val < 16 { 15 - val } else { val - 16 }
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct Noise {
    pub enabled: bool,
    pub length_value: u8,
    pub length_halt: bool,
    pub envelope: Envelope,
    pub timer: u16,
    pub timer_period: u16,
    pub mode: bool,
    pub shift_register: u16,
}

impl Noise {
    fn new() -> Self {
        Self {
            shift_register: 1,
            ..Default::default()
        }
    }

    fn write_ctrl(&mut self, value: u8) {
        self.length_halt = (value & 0x20) != 0;
        self.envelope.loop_mode = (value & 0x20) != 0;
        self.envelope.constant_volume = (value & 0x10) != 0;
        self.envelope.divider_period = value & 0x0F;
    }

    fn write_period(&mut self, value: u8) {
        self.mode = (value & 0x80) != 0;
        self.timer_period = NOISE_PERIOD_TABLE[(value & 0x0F) as usize];
    }

    fn write_length(&mut self, value: u8) {
        if self.enabled {
            self.length_value = LENGTH_TABLE[(value >> 3) as usize];
        }
        self.envelope.start = true;
    }

    fn step_timer(&mut self) {
        if self.timer == 0 {
            self.timer = self.timer_period;
            let shift = if self.mode { 6 } else { 1 };
            let bit1 = self.shift_register & 1;
            let bit2 = (self.shift_register >> shift) & 1;
            self.shift_register >>= 1;
            self.shift_register |= (bit1 ^ bit2) << 14;
        } else {
            self.timer -= 1;
        }
    }

    fn step_length(&mut self) {
        if !self.length_halt && self.length_value > 0 {
            self.length_value -= 1;
        }
    }

    fn step_envelope(&mut self) {
        self.envelope.step();
    }

    fn output(&self) -> u8 {
        if !self.enabled || self.length_value == 0 || (self.shift_register & 1) != 0 {
            0
        } else {
            self.envelope.output()
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct Dmc {
    pub enabled: bool,
    pub value: u8,
    pub sample_address: u16,
    pub sample_length: u16,
    pub current_address: u16,
    pub current_length: u16,
    pub shift_register: u8,
    pub bit_count: u8,
    pub timer: u16,
    pub timer_period: u16,
    pub irq_enabled: bool,
    pub loop_flag: bool,
    pub buffer: Option<u8>,
    pub irq_pending: bool,
}

impl Dmc {
    fn write_ctrl(&mut self, value: u8) {
        self.irq_enabled = (value & 0x80) != 0;
        self.loop_flag = (value & 0x40) != 0;
        self.timer_period = DMC_RATE_TABLE[(value & 0x0F) as usize];
        if !self.irq_enabled {
            self.irq_pending = false;
        }
    }

    fn write_direct(&mut self, value: u8) {
        self.value = value & 0x7F;
    }

    fn write_addr(&mut self, value: u8) {
        self.sample_address = 0xC000 + (value as u16 * 64);
    }

    fn write_length(&mut self, value: u8) {
        self.sample_length = (value as u16 * 16) + 1;
    }

    fn step_timer(&mut self) {
        if self.enabled {
            if self.timer == 0 {
                self.timer = self.timer_period;
                self.step_reader();
            } else {
                self.timer -= 1;
            }
        }
    }

    fn step_reader(&mut self) {
        if self.bit_count == 0 {
            self.bit_count = 8;
            if let Some(byte) = self.buffer.take() {
                self.shift_register = byte;
            } else {
                self.bit_count = 0;
            }
        }

        if self.bit_count > 0 {
            let bit = self.shift_register & 1;
            self.shift_register >>= 1;
            self.bit_count -= 1;

            if bit != 0 {
                if self.value <= 125 {
                    self.value += 2;
                }
            } else if self.value >= 2 {
                self.value -= 2;
            }
        }
    }

    fn restart(&mut self) {
        self.current_address = self.sample_address;
        self.current_length = self.sample_length;
    }
}

#[derive(Debug, Clone, Copy)]
pub struct HighPassFilter {
    c: f32,
    prev_out: f32,
    prev_in: f32,
}

impl HighPassFilter {
    fn new(hz: f32, sample_rate: f32) -> Self {
        let rc = 1.0 / (2.0 * std::f32::consts::PI * hz);
        let dt = 1.0 / sample_rate;
        let c = rc / (rc + dt);
        Self {
            c,
            prev_out: 0.0,
            prev_in: 0.0,
        }
    }

    fn step(&mut self, input: f32) -> f32 {
        let output = self.c * (self.prev_out + input - self.prev_in);
        self.prev_in = input;
        self.prev_out = output;
        output
    }
}

#[derive(Debug, Clone)]
pub struct Apu {
    pub status: ApuStatus,
    pub pulse1: Pulse,
    pub pulse2: Pulse,
    pub triangle: Triangle,
    pub noise: Noise,
    pub dmc: Dmc,
    pub frame_counter: usize,
    pub frame_mode: bool,
    pub frame_irq_inhibit: bool,
    pub frame_irq_pending: bool,
    pub cycles: usize,
    hpf1: HighPassFilter,
    hpf2: HighPassFilter,
}

impl Default for Apu {
    fn default() -> Self {
        Self {
            status: Default::default(),
            pulse1: Pulse::new(0),
            pulse2: Pulse::new(1),
            triangle: Default::default(),
            noise: Noise::new(),
            dmc: Default::default(),
            frame_counter: 0,
            frame_mode: false,
            frame_irq_inhibit: false,
            frame_irq_pending: false,
            cycles: 0,
            hpf1: HighPassFilter::new(90.0, 1789773.0),
            hpf2: HighPassFilter::new(440.0, 1789773.0),
        }
    }
}

impl Apu {
    pub fn write_register(&mut self, addr: u16, value: u8) {
        match addr {
            0x4000 => self.pulse1.write_ctrl(value),
            0x4001 => self.pulse1.write_sweep(value),
            0x4002 => self.pulse1.write_timer_low(value),
            0x4003 => self.pulse1.write_timer_high(value),
            0x4004 => self.pulse2.write_ctrl(value),
            0x4005 => self.pulse2.write_sweep(value),
            0x4006 => self.pulse2.write_timer_low(value),
            0x4007 => self.pulse2.write_timer_high(value),
            0x4008 => self.triangle.write_linear(value),
            0x400A => self.triangle.write_timer_low(value),
            0x400B => self.triangle.write_timer_high(value),
            0x400C => self.noise.write_ctrl(value),
            0x400E => self.noise.write_period(value),
            0x400F => self.noise.write_length(value),
            0x4010 => self.dmc.write_ctrl(value),
            0x4011 => self.dmc.write_direct(value),
            0x4012 => self.dmc.write_addr(value),
            0x4013 => self.dmc.write_length(value),
            0x4015 => self.write_status(value),
            0x4017 => self.write_frame_counter(value),
            _ => {}
        }
    }

    fn write_status(&mut self, value: u8) {
        self.status.set_enable_pulse1((value & 1) != 0);
        self.status.set_enable_pulse2((value & 2) != 0);
        self.status.set_enable_triangle((value & 4) != 0);
        self.status.set_enable_noise((value & 8) != 0);
        self.status.set_enable_dmc((value & 16) != 0);

        self.pulse1.enabled = self.status.enable_pulse1();
        if !self.pulse1.enabled {
            self.pulse1.length_value = 0;
        }

        self.pulse2.enabled = self.status.enable_pulse2();
        if !self.pulse2.enabled {
            self.pulse2.length_value = 0;
        }

        self.triangle.enabled = self.status.enable_triangle();
        if !self.triangle.enabled {
            self.triangle.length_value = 0;
        }

        self.noise.enabled = self.status.enable_noise();
        if !self.noise.enabled {
            self.noise.length_value = 0;
        }

        self.dmc.enabled = self.status.enable_dmc();
        if !self.dmc.enabled {
            self.dmc.current_length = 0;
        } else if self.dmc.current_length == 0 {
            self.dmc.restart();
        }
        self.dmc.irq_pending = false;
    }

    fn write_frame_counter(&mut self, value: u8) {
        self.frame_mode = (value & 0x80) != 0;
        self.frame_irq_inhibit = (value & 0x40) != 0;
        if self.frame_irq_inhibit {
            self.frame_irq_pending = false;
        }
        self.frame_counter = 0;
        if self.frame_mode {
            self.step_quarter_frame();
            self.step_half_frame();
        }
    }

    pub fn read_status(&mut self) -> u8 {
        let mut status = 0;
        if self.pulse1.length_value > 0 {
            status |= 1;
        }
        if self.pulse2.length_value > 0 {
            status |= 2;
        }
        if self.triangle.length_value > 0 {
            status |= 4;
        }
        if self.noise.length_value > 0 {
            status |= 8;
        }
        if self.dmc.current_length > 0 {
            status |= 16;
        }
        if self.frame_irq_pending {
            status |= 64;
        }
        if self.dmc.irq_pending {
            status |= 128;
        }
        self.frame_irq_pending = false;
        status
    }

    pub fn step(&mut self) {
        self.cycles += 1;

        if self.cycles.is_multiple_of(2) {
            self.pulse1.step_timer();
            self.pulse2.step_timer();
            self.noise.step_timer();
            self.dmc.step_timer();
        }
        self.triangle.step_timer();

        self.step_frame_counter();
    }

    fn step_frame_counter(&mut self) {
        self.frame_counter += 1;
        if !self.frame_mode {
            match self.frame_counter {
                7457 => self.step_quarter_frame(),
                14913 => {
                    self.step_quarter_frame();
                    self.step_half_frame();
                }
                22371 => self.step_quarter_frame(),
                29829 => {
                    self.step_quarter_frame();
                    self.step_half_frame();
                    if !self.frame_irq_inhibit {
                        self.frame_irq_pending = true;
                    }
                }
                29830 => {
                    if !self.frame_irq_inhibit {
                        self.frame_irq_pending = true;
                    }
                    self.frame_counter = 0;
                }
                _ => {}
            }
        } else {
            match self.frame_counter {
                7457 => self.step_quarter_frame(),
                14913 => {
                    self.step_quarter_frame();
                    self.step_half_frame();
                }
                22371 => self.step_quarter_frame(),
                29829 => {}
                37281 => {
                    self.step_quarter_frame();
                    self.step_half_frame();
                }
                37282 => {
                    self.frame_counter = 0;
                }
                _ => {}
            }
        }
    }

    fn step_quarter_frame(&mut self) {
        self.pulse1.step_envelope();
        self.pulse2.step_envelope();
        self.triangle.step_linear();
        self.noise.step_envelope();
    }

    fn step_half_frame(&mut self) {
        self.pulse1.step_length();
        self.pulse1.step_sweep();
        self.pulse2.step_length();
        self.pulse2.step_sweep();
        self.triangle.step_length();
        self.noise.step_length();
    }

    pub fn output(&mut self) -> f32 {
        let p1 = self.pulse1.output();
        let p2 = self.pulse2.output();
        let t = self.triangle.output();
        let n = self.noise.output();
        let d = self.dmc.value;

        let pulse_out = if p1 > 0 || p2 > 0 {
            95.88 / ((8128.0 / (p1 as f32 + p2 as f32)) + 100.0)
        } else {
            0.0
        };

        let tnd_out = if t > 0 || n > 0 || d > 0 {
            159.79
                / ((1.0 / ((t as f32 / 8227.0) + (n as f32 / 12241.0) + (d as f32 / 22638.0)))
                    + 100.0)
        } else {
            0.0
        };

        let mixed = pulse_out + tnd_out;

        let s1 = self.hpf1.step(mixed);
        self.hpf2.step(s1)
    }

    pub fn irq_occurred(&self) -> bool {
        self.frame_irq_pending || self.dmc.irq_pending
    }

    pub fn poll_dmc_dma(&mut self) -> Option<u16> {
        if self.dmc.enabled && self.dmc.current_length > 0 && self.dmc.buffer.is_none() {
            let addr = self.dmc.current_address;
            self.dmc.current_address = self.dmc.current_address.wrapping_add(1);
            if self.dmc.current_address == 0 {
                self.dmc.current_address = 0x8000;
            }
            self.dmc.current_length -= 1;
            if self.dmc.current_length == 0 {
                if self.dmc.loop_flag {
                    self.dmc.restart();
                } else if self.dmc.irq_enabled {
                    self.dmc.irq_pending = true;
                }
            }
            Some(addr)
        } else {
            None
        }
    }

    pub fn fill_dmc_buffer(&mut self, val: u8) {
        self.dmc.buffer = Some(val);
    }
}
