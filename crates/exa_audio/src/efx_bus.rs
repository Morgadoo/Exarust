//! The reverb slot as a Bevy audio source: the auxiliary send the original
//! routes every world voice to, mixed and reverberated on rodio's audio
//! thread next to the dry sources.
//!
//! Bevy's rodio sinks cannot tap a playing source, so the bus keeps a wet
//! copy of each source slot that has a send: the same buffer (decoded once
//! from the bank's `AudioSource`), started, paused, resumed, restarted and
//! stopped by the same mixer commands as the dry entity, at the same pitch,
//! with the send gain and band gains of [`efx::wet_send`]. Every sample the
//! bus sums the wet copies and runs them through [`efx::ReverbDsp`] holding
//! the block the effect slot holds ([`Reverb::effect`](crate::Reverb::effect)).
//!
//! As in the original, a source's send is decided when it is bound: a voice
//! bound while the reverb was on keeps sending until it is rebound.
//!
//! Flags: `EXA_EFX_LOG=1` logs the send and reverb output levels once a
//! second; `EXA_EFX_RECORD=<path.wav>` records the reverb's output (wet
//! only, 32-bit float stereo at 48 kHz) for `EXA_EFX_RECORD_SECS` seconds
//! (default 30) from the first effect upload.

use std::collections::HashMap;
use std::fs::File;
use std::io::{Seek, SeekFrom, Write};
use std::num::NonZero;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use bevy::asset::AssetId;
use bevy::audio::{Decodable, Source};
use bevy::prelude::*;

use crate::SoundBank;
use crate::efx::{self, Biquad, ReverbDsp, Shelf, WetSend};
use crate::environment::{AudioEnvironment, SOURCE_CAPACITY, SourceCommand};
use crate::reverb::EaxReverb;

/// The bus's sample rate; rodio converts it to the device's.
pub const BUS_RATE: u32 = 48_000;
/// Frames between two polls of the command channel.
const POLL_FRAMES: u32 = 64;
/// Frames in one level-meter window: 0.1 s.
const METER_FRAMES: u32 = BUS_RATE / 10;

/// A buffer's samples summed to one channel (the omnidirectional part of
/// its send) and its rate.
#[derive(Debug)]
pub struct Pcm {
    samples: Vec<f32>,
    rate: u32,
}

impl Pcm {
    /// Sums interleaved `channels`-channel samples to one channel.
    pub fn from_interleaved(samples: &[f32], channels: usize, rate: u32) -> Self {
        let channels = channels.max(1);
        Self {
            samples: samples
                .chunks(channels)
                .map(|frame| frame.iter().sum())
                .collect(),
            rate: rate.max(1),
        }
    }
}

/// What the main thread tells the bus.
#[derive(Debug)]
pub enum BusCommand {
    /// The slot's effect block changed (an upload).
    Effect(EaxReverb),
    /// A source slot starts a buffer with a send.
    Play {
        slot: u8,
        pcm: Arc<Pcm>,
        offset_s: f32,
        pitch: f32,
        looping: bool,
        paused: bool,
        restart: bool,
        send: WetSend,
    },
    /// A sending slot's pitch and send changed.
    Update {
        slot: u8,
        pitch: f32,
        send: WetSend,
    },
    Pause {
        slot: u8,
    },
    Resume {
        slot: u8,
    },
    /// The slot stops (or rebinds without a send).
    Stop {
        slot: u8,
    },
}

/// The wet copy of one source slot.
#[derive(Debug)]
struct BusVoice {
    pcm: Arc<Pcm>,
    position: f64,
    step: f64,
    pitch: f32,
    looping: bool,
    paused: bool,
    finished: bool,
    gain: f32,
    hf: Biquad,
    lf: Biquad,
}

impl BusVoice {
    fn new(pcm: Arc<Pcm>, offset_s: f32, pitch: f32, looping: bool, paused: bool) -> Self {
        let step = f64::from(pcm.rate) / f64::from(BUS_RATE);
        let position = f64::from(offset_s.max(0.0)) * f64::from(pcm.rate);
        Self {
            pcm,
            position,
            step,
            pitch,
            looping,
            paused,
            finished: false,
            gain: 0.0,
            hf: Biquad::default(),
            lf: Biquad::default(),
        }
    }

    fn set_send(&mut self, send: WetSend) {
        let rate = BUS_RATE as f32;
        self.gain = send.gain;
        self.hf.set_shelf(
            Shelf::High,
            efx::SEND_HF_REFERENCE_HZ / rate,
            send.gain_hf.max(efx::DECAY_GAIN),
        );
        self.lf.set_shelf(
            Shelf::Low,
            efx::SEND_LF_REFERENCE_HZ / rate,
            send.gain_lf.max(efx::DECAY_GAIN),
        );
    }

    /// The next wet sample (linear interpolation at the pitch).
    fn next(&mut self) -> f32 {
        if self.paused || self.finished {
            return 0.0;
        }
        let len = self.pcm.samples.len();
        if len == 0 {
            self.finished = true;
            return 0.0;
        }
        let index = self.position as usize;
        if index >= len {
            self.finished = true;
            return 0.0;
        }
        let frac = (self.position - index as f64) as f32;
        let a = self.pcm.samples[index];
        let b = match self.pcm.samples.get(index + 1) {
            Some(b) => *b,
            None if self.looping => self.pcm.samples[0],
            None => 0.0,
        };
        let sample = a + (b - a) * frac;
        self.position += self.step * f64::from(self.pitch.max(0.0));
        if self.position >= len as f64 {
            if self.looping {
                self.position %= len as f64;
            } else {
                self.finished = true;
            }
        }
        self.lf.process(self.hf.process(sample * self.gain))
    }
}

/// The bus's level meter, written on the audio thread.
#[derive(Debug, Default)]
pub struct BusMeter {
    /// The loudest 0.1 s RMS of the summed send since the last read (bits).
    send_rms: AtomicU32,
    /// The loudest 0.1 s RMS of the reverb's output since the last read.
    out_rms: AtomicU32,
    /// Sending voices at the last window.
    voices: AtomicU32,
    /// Frames rendered.
    frames: AtomicU64,
}

impl BusMeter {
    /// Takes the loudest send and output RMS since the last call, the
    /// sending voices and the frames rendered.
    pub fn take(&self) -> (f32, f32, u32, u64) {
        (
            f32::from_bits(self.send_rms.swap(0, Ordering::Relaxed)),
            f32::from_bits(self.out_rms.swap(0, Ordering::Relaxed)),
            self.voices.load(Ordering::Relaxed),
            self.frames.load(Ordering::Relaxed),
        )
    }
}

/// The bus's audio-thread half: a rodio source of the reverb's stereo
/// output.
pub struct ReverbBusDecoder {
    rx: Receiver<BusCommand>,
    voices: Vec<Option<BusVoice>>,
    dsp: Option<ReverbDsp>,
    frame: [f32; 2],
    channel: usize,
    until_poll: u32,
    meter: Arc<BusMeter>,
    window: u32,
    send_sq: f64,
    out_sq: f64,
    record: Option<Arc<Mutex<Vec<f32>>>>,
    pending: Vec<f32>,
}

impl ReverbBusDecoder {
    fn new(
        rx: Receiver<BusCommand>,
        meter: Arc<BusMeter>,
        record: Option<Arc<Mutex<Vec<f32>>>>,
    ) -> Self {
        Self {
            rx,
            voices: (0..SOURCE_CAPACITY).map(|_| None).collect(),
            dsp: None,
            frame: [0.0; 2],
            channel: 0,
            until_poll: 0,
            meter,
            window: 0,
            send_sq: 0.0,
            out_sq: 0.0,
            record,
            pending: Vec::new(),
        }
    }

    fn apply(&mut self, command: BusCommand) {
        match command {
            BusCommand::Effect(block) => match &mut self.dsp {
                Some(dsp) => dsp.update(&block),
                None => self.dsp = Some(ReverbDsp::new(BUS_RATE as f32, &block)),
            },
            BusCommand::Play {
                slot,
                pcm,
                offset_s,
                pitch,
                looping,
                paused,
                restart,
                send,
            } => {
                let Some(entry) = self.voices.get_mut(slot as usize) else {
                    return;
                };
                if restart && let Some(voice) = entry.as_mut().filter(|v| !v.finished) {
                    // As the dry source: a paused one resumes, a playing one
                    // starts its old buffer again.
                    if voice.paused {
                        voice.paused = false;
                    } else {
                        voice.position = 0.0;
                    }
                    voice.pitch = pitch;
                    voice.set_send(send);
                    return;
                }
                let mut voice = BusVoice::new(pcm, offset_s, pitch, looping, paused);
                voice.set_send(send);
                *entry = Some(voice);
            }
            BusCommand::Update { slot, pitch, send } => {
                if let Some(Some(voice)) = self.voices.get_mut(slot as usize) {
                    voice.pitch = pitch;
                    voice.set_send(send);
                }
            }
            BusCommand::Pause { slot } | BusCommand::Resume { slot } => {
                let pause = matches!(command, BusCommand::Pause { .. });
                if let Some(Some(voice)) = self.voices.get_mut(slot as usize) {
                    voice.paused = pause;
                }
            }
            BusCommand::Stop { slot } => {
                if let Some(entry) = self.voices.get_mut(slot as usize) {
                    *entry = None;
                }
            }
        }
    }

    /// Renders one stereo frame.
    pub fn render(&mut self) -> [f32; 2] {
        let mut send = 0.0;
        let mut sending = 0;
        for entry in &mut self.voices {
            if let Some(voice) = entry {
                send += voice.next();
                if voice.finished {
                    *entry = None;
                } else if !voice.paused {
                    sending += 1;
                }
            }
        }
        let out = match &mut self.dsp {
            Some(dsp) => dsp.process(send),
            None => [0.0; 2],
        };
        self.send_sq += f64::from(send * send);
        self.out_sq += f64::from(out[0] * out[0] + out[1] * out[1]) * 0.5;
        self.window += 1;
        if self.window >= METER_FRAMES {
            let n = f64::from(self.window);
            let send_rms = (self.send_sq / n).sqrt() as f32;
            let out_rms = (self.out_sq / n).sqrt() as f32;
            self.meter
                .send_rms
                .fetch_max(send_rms.to_bits(), Ordering::Relaxed);
            self.meter
                .out_rms
                .fetch_max(out_rms.to_bits(), Ordering::Relaxed);
            self.meter.voices.store(sending, Ordering::Relaxed);
            self.meter
                .frames
                .fetch_add(u64::from(self.window), Ordering::Relaxed);
            self.window = 0;
            self.send_sq = 0.0;
            self.out_sq = 0.0;
        }
        if self.record.is_some() {
            self.pending.extend_from_slice(&out);
        }
        out
    }

    fn poll(&mut self) {
        while let Ok(command) = self.rx.try_recv() {
            self.apply(command);
        }
        if let Some(record) = &self.record
            && !self.pending.is_empty()
            && let Ok(mut shared) = record.try_lock()
        {
            shared.append(&mut self.pending);
        }
    }
}

impl Iterator for ReverbBusDecoder {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        if self.channel == 0 {
            if self.until_poll == 0 {
                self.poll();
                self.until_poll = POLL_FRAMES;
            }
            self.until_poll -= 1;
            self.frame = self.render();
        }
        let sample = self.frame[self.channel];
        self.channel ^= 1;
        Some(sample)
    }
}

impl Source for ReverbBusDecoder {
    fn current_span_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> NonZero<u16> {
        NonZero::new(2).unwrap_or(NonZero::<u16>::MIN)
    }

    fn sample_rate(&self) -> NonZero<u32> {
        NonZero::new(BUS_RATE).unwrap_or(NonZero::<u32>::MIN)
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

/// The bus as a Bevy audio asset; its decoder is handed out once.
#[derive(Asset, TypePath)]
pub struct ReverbBusAudio {
    decoder: Mutex<Option<ReverbBusDecoder>>,
}

/// The decoder of [`ReverbBusAudio`]: the bus, or an ended source when
/// the asset is played a second time.
pub struct BusSource(Option<ReverbBusDecoder>);

impl Iterator for BusSource {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        self.0.as_mut()?.next()
    }
}

impl Source for BusSource {
    fn current_span_len(&self) -> Option<usize> {
        match &self.0 {
            Some(_) => None,
            None => Some(0),
        }
    }

    fn channels(&self) -> NonZero<u16> {
        NonZero::new(2).unwrap_or(NonZero::<u16>::MIN)
    }

    fn sample_rate(&self) -> NonZero<u32> {
        NonZero::new(BUS_RATE).unwrap_or(NonZero::<u32>::MIN)
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

impl Decodable for ReverbBusAudio {
    type Decoder = BusSource;

    fn decoder(&self) -> BusSource {
        BusSource(self.decoder.lock().ok().and_then(|mut d| d.take()))
    }
}

/// The f32 WAV the reverb's output is recorded to.
struct Recorder {
    file: File,
    shared: Arc<Mutex<Vec<f32>>>,
    written: u64,
    limit: u64,
    started: bool,
}

impl Recorder {
    fn header(data_bytes: u32) -> Vec<u8> {
        let mut h = Vec::with_capacity(44);
        h.extend_from_slice(b"RIFF");
        h.extend_from_slice(&(36 + data_bytes).to_le_bytes());
        h.extend_from_slice(b"WAVEfmt ");
        h.extend_from_slice(&16u32.to_le_bytes());
        h.extend_from_slice(&3u16.to_le_bytes());
        h.extend_from_slice(&2u16.to_le_bytes());
        h.extend_from_slice(&BUS_RATE.to_le_bytes());
        h.extend_from_slice(&(BUS_RATE * 8).to_le_bytes());
        h.extend_from_slice(&8u16.to_le_bytes());
        h.extend_from_slice(&32u16.to_le_bytes());
        h.extend_from_slice(b"data");
        h.extend_from_slice(&data_bytes.to_le_bytes());
        h
    }

    fn drain(&mut self) {
        let samples = match self.shared.lock() {
            Ok(mut shared) => std::mem::take(&mut *shared),
            Err(_) => return,
        };
        if !self.started || self.written >= self.limit {
            return;
        }
        let room = (self.limit - self.written) as usize;
        let take = samples.len().min(room);
        let bytes: Vec<u8> = samples[..take]
            .iter()
            .flat_map(|s| s.to_le_bytes())
            .collect();
        let ok = self.file.seek(SeekFrom::End(0)).is_ok() && self.file.write_all(&bytes).is_ok();
        if !ok {
            warn!("audio efx: the reverb recording could not be written");
            self.written = self.limit;
            return;
        }
        self.written += take as u64;
        let data = (self.written * 4).min(u64::from(u32::MAX - 36)) as u32;
        let _ = self.file.seek(SeekFrom::Start(0));
        let _ = self.file.write_all(&Self::header(data));
        if self.written >= self.limit {
            info!(
                "audio efx: recorded {:.1} s of the reverb's output",
                self.written as f64 / 2.0 / f64::from(BUS_RATE)
            );
        }
    }
}

/// The bus's main-thread half.
#[derive(Resource)]
pub(crate) struct ReverbBus {
    tx: Sender<BusCommand>,
    pcm: HashMap<AssetId<AudioSource>, Arc<Pcm>>,
    sending: [bool; SOURCE_CAPACITY],
    uploads: Option<u32>,
    meter: Arc<BusMeter>,
    log_levels: bool,
    next_log_s: f64,
    record: Option<Recorder>,
}

/// Spawns the reverb bus's player once the audio output exists.
pub(crate) fn spawn_reverb_bus(mut commands: Commands, mut assets: ResMut<Assets<ReverbBusAudio>>) {
    let (tx, rx) = channel();
    let meter = Arc::new(BusMeter::default());
    let record = std::env::var("EXA_EFX_RECORD").ok().and_then(|path| {
        let secs: f64 = std::env::var("EXA_EFX_RECORD_SECS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(30.0);
        match File::create(&path) {
            Ok(mut file) => {
                let _ = file.write_all(&Recorder::header(0));
                info!("audio efx: recording the reverb's output to {path} ({secs} s)");
                Some(Recorder {
                    file,
                    shared: Arc::new(Mutex::new(Vec::new())),
                    written: 0,
                    limit: (secs.max(0.0) * f64::from(BUS_RATE) * 2.0) as u64,
                    started: false,
                })
            }
            Err(error) => {
                warn!("audio efx: cannot record to {path}: {error}");
                None
            }
        }
    });
    let decoder =
        ReverbBusDecoder::new(rx, meter.clone(), record.as_ref().map(|r| r.shared.clone()));
    let handle = assets.add(ReverbBusAudio {
        decoder: Mutex::new(Some(decoder)),
    });
    commands.spawn((
        AudioPlayer(handle),
        PlaybackSettings::ONCE,
        Name::new("reverb bus"),
    ));
    commands.insert_resource(ReverbBus {
        tx,
        pcm: HashMap::new(),
        sending: [false; SOURCE_CAPACITY],
        uploads: None,
        meter,
        log_levels: std::env::var("EXA_EFX_LOG").is_ok_and(|v| v != "0"),
        next_log_s: 0.0,
        record,
    });
}

/// Decodes a bank buffer to the bus's mono samples.
fn decode(source: &AudioSource) -> Pcm {
    let decoder = source.decoder();
    let channels = usize::from(decoder.channels().get());
    let rate = decoder.sample_rate().get();
    let samples: Vec<f32> = decoder.collect();
    Pcm::from_interleaved(&samples, channels, rate)
}

/// Mirrors the frame's source commands onto the bus with each sending
/// slot's wet gains, and uploads the effect when it changed.
pub(crate) fn drive_reverb_bus(
    env: Res<AudioEnvironment>,
    bank: Res<SoundBank>,
    sources: Option<Res<Assets<AudioSource>>>,
    bus: Option<ResMut<ReverbBus>>,
    real: Res<Time<Real>>,
) {
    let Some(mut bus) = bus else {
        return;
    };
    let bus = &mut *bus;
    if env.reverb.has_effect() && bus.uploads != Some(env.reverb.uploads()) {
        bus.uploads = Some(env.reverb.uploads());
        let block = env.reverb.effect();
        info!(
            "audio efx: slot effect EAX reverb: density {} diffusion {} gain {:.4} gain HF {:.4} gain LF {:.4} decay {:.3} s (HF ratio {:.2}, LF ratio {:.2}) reflections {:.3} at {:.3} s, late {:.4} at {:.3} s, air absorption HF {:.4}, references {} Hz / {} Hz, room rolloff {}, HF limit {}; reverb {}",
            block.density,
            block.diffusion,
            block.gain,
            block.gain_hf,
            block.gain_lf,
            block.decay_s,
            block.decay_hf_ratio,
            block.decay_lf_ratio,
            block.reflections_gain,
            block.reflections_delay_s,
            block.late_gain,
            block.late_delay_s,
            block.air_absorption_hf,
            block.hf_reference_hz,
            block.lf_reference_hz,
            block.room_rolloff,
            block.decay_hf_limit,
            if env.reverb.enabled { "on" } else { "off" }
        );
        let _ = bus.tx.send(BusCommand::Effect(block));
        if let Some(record) = &mut bus.record {
            record.started = true;
        }
    }
    let listener = env.listener.position();
    let effect = env.reverb.effect();
    for command in env.commands() {
        let message = match *command {
            SourceCommand::Play(play) => {
                let slot = play.slot as usize;
                let pcm = play
                    .reverb
                    .then(|| bank.buffer_handle(play.buffer))
                    .flatten()
                    .and_then(|handle| {
                        if let Some(pcm) = bus.pcm.get(&handle.id()) {
                            return Some(pcm.clone());
                        }
                        let source = sources.as_ref()?.get(&handle)?;
                        let pcm = Arc::new(decode(source));
                        bus.pcm.insert(handle.id(), pcm.clone());
                        Some(pcm)
                    });
                if let Some(sending) = bus.sending.get_mut(slot) {
                    *sending = pcm.is_some();
                }
                match pcm {
                    Some(pcm) => BusCommand::Play {
                        slot: play.slot,
                        pcm,
                        offset_s: play.offset,
                        pitch: play.pitch,
                        looping: play.looping,
                        paused: play.paused,
                        restart: play.restart,
                        send: efx::wet_send(
                            play.gain,
                            play.min_gain,
                            play.position,
                            listener,
                            &effect,
                        ),
                    },
                    None => BusCommand::Stop { slot: play.slot },
                }
            }
            SourceCommand::Refresh(update) => {
                if !bus
                    .sending
                    .get(update.slot as usize)
                    .copied()
                    .unwrap_or(false)
                {
                    continue;
                }
                BusCommand::Update {
                    slot: update.slot,
                    pitch: update.pitch,
                    send: efx::wet_send(
                        update.gain,
                        update.min_gain,
                        update.position,
                        listener,
                        &effect,
                    ),
                }
            }
            SourceCommand::Pause { slot } => BusCommand::Pause { slot },
            SourceCommand::Resume { slot } => BusCommand::Resume { slot },
            SourceCommand::Stop { slot } => {
                if let Some(sending) = bus.sending.get_mut(slot as usize) {
                    *sending = false;
                }
                BusCommand::Stop { slot }
            }
        };
        let _ = bus.tx.send(message);
    }
    if let Some(record) = &mut bus.record {
        record.drain();
    }
    let now = real.elapsed_secs_f64();
    if bus.log_levels && now >= bus.next_log_s {
        bus.next_log_s = now + 1.0;
        let (send, out, voices, frames) = bus.meter.take();
        let db = |v: f32| {
            if v > 0.0 {
                format!("{:.1} dBFS", 20.0 * v.log10())
            } else {
                "silent".to_owned()
            }
        };
        info!(
            "audio efx levels: send rms {send:.5} ({}), reverb out rms {out:.5} ({}), sending voices {voices}, slots with a send {}, reverb {} (effect decay {:.3} s), frames {frames}",
            db(send),
            db(out),
            bus.sending.iter().filter(|s| **s).count(),
            if env.reverb.enabled { "on" } else { "off" },
            effect.decay_s
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decoder() -> (Sender<BusCommand>, ReverbBusDecoder) {
        let (tx, rx) = channel();
        (tx, ReverbBusDecoder::new(rx, Arc::default(), None))
    }

    fn send(gain: f32) -> WetSend {
        WetSend {
            gain,
            gain_hf: 1.0,
            gain_lf: 1.0,
        }
    }

    fn run(decoder: &mut ReverbBusDecoder, frames: usize) -> f32 {
        let mut energy = 0.0;
        for _ in 0..frames * 2 {
            let s = decoder.next().unwrap_or(0.0);
            energy += s * s;
        }
        energy
    }

    /// Stereo frames interleave; without an effect the bus is silent.
    #[test]
    fn the_bus_is_silent_until_the_effect_is_attached() {
        let (tx, mut bus) = decoder();
        let pcm = Arc::new(Pcm::from_interleaved(&[0.5; 4800], 1, 48_000));
        tx.send(BusCommand::Play {
            slot: 0,
            pcm,
            offset_s: 0.0,
            pitch: 1.0,
            looping: true,
            paused: false,
            restart: false,
            send: send(1.0),
        })
        .unwrap();
        assert_eq!(run(&mut bus, 4800), 0.0);
        tx.send(BusCommand::Effect(EaxReverb::default())).unwrap();
        assert!(run(&mut bus, 9600) > 0.0);
    }

    /// A sending voice reverberates; a stopped one leaves only the tail,
    /// and a zero send gives nothing.
    #[test]
    fn a_send_feeds_the_reverb_and_stop_leaves_the_tail() {
        let (tx, mut bus) = decoder();
        tx.send(BusCommand::Effect(EaxReverb::default())).unwrap();
        let noise: Vec<f32> = (0..4800)
            .map(|i| ((i * 7919) % 200) as f32 / 100.0 - 1.0)
            .collect();
        let pcm = Arc::new(Pcm::from_interleaved(&noise, 1, 48_000));
        tx.send(BusCommand::Play {
            slot: 3,
            pcm: pcm.clone(),
            offset_s: 0.0,
            pitch: 1.0,
            looping: true,
            paused: false,
            restart: false,
            send: send(0.0),
        })
        .unwrap();
        assert_eq!(run(&mut bus, 9600), 0.0);
        tx.send(BusCommand::Update {
            slot: 3,
            pitch: 1.0,
            send: send(1.0),
        })
        .unwrap();
        let wet = run(&mut bus, 9600);
        assert!(wet > 0.0);
        tx.send(BusCommand::Stop { slot: 3 }).unwrap();
        let tail = run(&mut bus, 9600);
        assert!(tail > 0.0, "{tail} {wet}");
        run(&mut bus, 48_000 * 4);
        let later = run(&mut bus, 4800);
        assert!(later < tail * 1e-4, "{later} {tail}");
    }

    /// A one-shot ends on its own; stereo buffers sum their channels.
    #[test]
    fn a_one_shot_ends_and_stereo_sums() {
        let pcm = Pcm::from_interleaved(&[0.25, 0.5, 0.25, 0.5], 2, 22_050);
        assert_eq!(pcm.samples, vec![0.75, 0.75]);
        let mut voice = BusVoice::new(Arc::new(pcm), 0.0, 1.0, false, false);
        voice.set_send(send(1.0));
        let mut n = 0;
        while !voice.finished && n < 100 {
            voice.next();
            n += 1;
        }
        assert!(voice.finished);
        // Two source frames at 22 050 Hz span about 4.35 bus frames.
        assert!((4..=5).contains(&n), "{n}");
    }
}
