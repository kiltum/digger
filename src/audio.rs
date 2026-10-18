//! cpal output stream + a silent fallback driver.
//!
//! The sound engine is a faithful PC-speaker emulator (see `sound.rs`).  It is
//! advanced by whatever thread renders samples, so if no audio device exists we
//! still run a silent driver to keep music/effect timing moving.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SampleFormat, SizedSample, StreamConfig};

use crate::sound::SoundEngine;

pub struct Audio {
    pub engine: Arc<Mutex<SoundEngine>>,
    stream: Option<cpal::Stream>,
    stop: Arc<AtomicBool>,
}

impl Drop for Audio {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        self.stream.take();
    }
}

pub fn start() -> Audio {
    let stop = Arc::new(AtomicBool::new(false));
    match try_open() {
        Ok((engine, stream)) => Audio {
            engine,
            stream: Some(stream),
            stop,
        },
        Err(err) => {
            eprintln!("audio: no output device ({err}); running silently");
            let engine = Arc::new(Mutex::new(SoundEngine::new(48_000)));
            let silent_engine = engine.clone();
            let silent_stop = stop.clone();
            std::thread::Builder::new()
                .name("digger-silent-audio".into())
                .spawn(move || {
                    let mut buf = vec![0.0f32; 480];
                    while !silent_stop.load(Ordering::Relaxed) {
                        if let Ok(mut e) = silent_engine.lock() {
                            e.render(&mut buf);
                        }
                        std::thread::sleep(std::time::Duration::from_millis(10));
                    }
                })
                .ok();
            Audio {
                engine,
                stream: None,
                stop,
            }
        }
    }
}

fn try_open() -> Result<(Arc<Mutex<SoundEngine>>, cpal::Stream), String> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or_else(|| "no default output device".to_string())?;
    let supported = device
        .default_output_config()
        .map_err(|e| format!("default_output_config: {e}"))?;
    let sample_rate = supported.sample_rate();
    let config: StreamConfig = supported.into();
    let engine = Arc::new(Mutex::new(SoundEngine::new(sample_rate)));
    let stream = match supported.sample_format() {
        SampleFormat::F32 => build::<f32>(&device, config, engine.clone()),
        SampleFormat::I16 => build::<i16>(&device, config, engine.clone()),
        SampleFormat::U16 => build::<u16>(&device, config, engine.clone()),
        SampleFormat::I32 => build::<i32>(&device, config, engine.clone()),
        SampleFormat::U32 => build::<u32>(&device, config, engine.clone()),
        SampleFormat::I8 => build::<i8>(&device, config, engine.clone()),
        SampleFormat::U8 => build::<u8>(&device, config, engine.clone()),
        other => return Err(format!("unsupported sample format {other}")),
    }?;
    stream.play().map_err(|e| format!("play: {e}"))?;
    Ok((engine, stream))
}

fn build<T>(
    device: &cpal::Device,
    config: StreamConfig,
    engine: Arc<Mutex<SoundEngine>>,
) -> Result<cpal::Stream, String>
where
    T: SizedSample + FromSample<f32>,
{
    let channels = config.channels as usize;
    let mut mono: Vec<f32> = Vec::new();
    let err_fn = |err| eprintln!("audio stream error: {err}");
    device
        .build_output_stream(
            config,
            move |out: &mut [T], _info: &cpal::OutputCallbackInfo| {
                let frames = out.len() / channels.max(1);
                if mono.len() < frames {
                    mono.resize(frames, 0.0);
                }
                if let Ok(mut e) = engine.lock() {
                    e.render(&mut mono[..frames]);
                } else {
                    mono[..frames].fill(0.0);
                }
                for (chunk, &s) in out.chunks_mut(channels.max(1)).zip(mono.iter()) {
                    let v = T::from_sample(s);
                    for c in chunk.iter_mut() {
                        *c = v;
                    }
                }
            },
            err_fn,
            None,
        )
        .map_err(|e| format!("build_output_stream: {e}"))
}
