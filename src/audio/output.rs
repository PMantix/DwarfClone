//! The sound device (through cpal), on a thread of its own. The device's
//! callback drains the messages and runs the engine; the game only ever
//! sends on a channel, which never blocks. If there's no device, or it
//! won't open, the thread just ends and the messages go nowhere.

use super::engine::Engine;
use super::{Backend, Msg};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::SampleFormat;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;

pub struct DeviceSink {
    tx: Sender<Msg>,
    /// Dropped when the game ends, which lets the thread close the stream.
    _stop: Sender<()>,
    alive: Arc<AtomicBool>,
    /// Frames the device has asked for so far (for `--sound-check`).
    pub frames: Arc<AtomicU64>,
}

impl Backend for DeviceSink {
    fn send(&mut self, m: Msg) {
        let _ = self.tx.send(m);
    }

    fn live(&self) -> bool {
        self.alive.load(Ordering::Relaxed)
    }
}

pub fn open(seed: u32) -> Option<DeviceSink> {
    let (tx, rx) = channel();
    let (stop_tx, stop_rx) = channel::<()>();
    // Live until the device is known to have failed.
    let alive = Arc::new(AtomicBool::new(true));
    let frames = Arc::new(AtomicU64::new(0));
    let (flag, count) = (alive.clone(), frames.clone());
    std::thread::Builder::new()
        .name("audio".into())
        .spawn(move || {
            if run(rx, stop_rx, seed, flag.clone(), count).is_none() {
                flag.store(false, Ordering::Relaxed);
            }
        })
        .ok()?;
    Some(DeviceSink { tx, _stop: stop_tx, alive, frames })
}

fn drain(rx: &Receiver<Msg>, e: &mut Engine) {
    while let Ok(m) = rx.try_recv() {
        e.handle(m);
    }
}

/// The default output device's name.
pub fn device_name() -> Option<String> {
    cpal::default_host().default_output_device().and_then(|d| d.name().ok())
}

fn run(rx: Receiver<Msg>, stop: Receiver<()>, seed: u32, alive: Arc<AtomicBool>, frames: Arc<AtomicU64>) -> Option<()> {
    let host = cpal::default_host();
    let dev = host.default_output_device()?;
    let supported = dev.default_output_config().ok()?;
    let format = supported.sample_format();
    let config: cpal::StreamConfig = supported.into();
    let ch = config.channels as usize;
    let mut engine = Engine::new(config.sample_rate.0 as f32, seed);
    let failed = alive.clone();
    let on_error = move |_e: cpal::StreamError| failed.store(false, Ordering::Relaxed);
    let mut scratch: Vec<f32> = Vec::new();
    let stream = match format {
        SampleFormat::F32 => dev.build_output_stream(
            &config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                drain(&rx, &mut engine);
                engine.render(data, ch);
                frames.fetch_add((data.len() / ch.max(1)) as u64, Ordering::Relaxed);
            },
            on_error,
            None,
        ),
        SampleFormat::I16 => dev.build_output_stream(
            &config,
            move |data: &mut [i16], _: &cpal::OutputCallbackInfo| {
                drain(&rx, &mut engine);
                scratch.resize(data.len(), 0.0);
                engine.render(&mut scratch, ch);
                for (o, s) in data.iter_mut().zip(scratch.iter()) {
                    *o = (s * 32767.0) as i16;
                }
                frames.fetch_add((data.len() / ch.max(1)) as u64, Ordering::Relaxed);
            },
            on_error,
            None,
        ),
        SampleFormat::U16 => dev.build_output_stream(
            &config,
            move |data: &mut [u16], _: &cpal::OutputCallbackInfo| {
                drain(&rx, &mut engine);
                scratch.resize(data.len(), 0.0);
                engine.render(&mut scratch, ch);
                for (o, s) in data.iter_mut().zip(scratch.iter()) {
                    *o = (s * 32767.0 + 32768.0) as u16;
                }
                frames.fetch_add((data.len() / ch.max(1)) as u64, Ordering::Relaxed);
            },
            on_error,
            None,
        ),
        _ => return None,
    }
    .ok()?;
    stream.play().ok()?;
    // Hold the stream until the game ends (the sender is dropped).
    let _ = stop.recv();
    drop(stream);
    Some(())
}
