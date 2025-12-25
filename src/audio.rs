use anyhow::Context;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use ringbuf::HeapCons;
use ringbuf::traits::Consumer;

pub struct Audio {
    _stream: cpal::Stream,
    pub sample_rate: f32,
}

impl Audio {
    pub fn new(mut consumer: HeapCons<f32>) -> Result<Self, anyhow::Error> {
        let host = cpal::default_host();

        let device = host
            .default_output_device()
            .context("No output device available")?;

        let config: cpal::StreamConfig = device.default_output_config()?.into();
        let sample_rate = config.sample_rate.0 as f32;
        let channels = config.channels as usize;
        let err_fn = |err| eprintln!("An error occurred on stream: {}", err);

        let stream = device.build_output_stream(
            &config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                for frame in data.chunks_mut(channels) {
                    let sample = consumer.try_pop().unwrap_or(0.0);
                    for point in frame.iter_mut() {
                        *point = sample;
                    }
                }
            },
            err_fn,
            None,
        )?;

        stream.play().context("Failed to play stream")?;

        Ok(Self {
            _stream: stream,
            sample_rate,
        })
    }
}
