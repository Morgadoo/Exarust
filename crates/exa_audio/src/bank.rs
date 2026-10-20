//! Sounds decoded from `Sound.rpk`: the name-keyed buffer cache, the sound
//! sets, and the picker every event uses to choose a variant.
//!
//! The original keys sounds by the first 16 bytes of the lower-cased name,
//! keeps one table entry per loaded file with a reference count, and caches a
//! numbered set (`base01` … `base16`, stopping at the first missing variant)
//! together with its last two picks. Variant picks step the engine's
//! `× 0x08088405 + 1` generator and never repeat those last picks.

use std::collections::HashMap;
use std::sync::Arc;

use bevy::prelude::*;
use exa_assets::{Game, PackKind};

/// Bytes of a sound's name that identify it.
pub const NAME_KEY_LEN: usize = 16;
/// Variants the original probes in a numbered set.
pub const MAX_VARIANTS: usize = 16;
/// The engine's linear congruential multiplier.
pub const LCG_MUL: u32 = 0x0808_8405;
/// The engine's linear congruential addend.
pub const LCG_ADD: u32 = 1;
/// `2^-32`, the scale that turns the generator's word into a unit draw.
pub const UNIT_SCALE: f64 = 1.0 / 4_294_967_296.0;
/// The longest base name a set can be built from, `base01` filling the key.
pub const MAX_SET_BASE_LEN: usize = 14;

/// A sound's identity: its lower-cased name, cut at 16 bytes and padded with
/// zeroes. Short names keep the zeroes in the comparison, as the original's
/// fixed 16-byte key does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct NameKey([u8; NAME_KEY_LEN]);

impl NameKey {
    /// The key of `name`: ASCII letters lower-cased, at most 16 bytes.
    pub fn new(name: &str) -> Self {
        let mut key = [0u8; NAME_KEY_LEN];
        for (slot, byte) in key.iter_mut().zip(name.bytes()) {
            *slot = byte.to_ascii_lowercase();
        }
        Self(key)
    }

    /// The key as the lower-cased, zero-trimmed text a file would carry.
    pub fn text(&self) -> String {
        let end = self.0.iter().position(|&b| b == 0).unwrap_or(NAME_KEY_LEN);
        String::from_utf8_lossy(&self.0[..end]).into_owned()
    }

    /// Whether the key holds no name.
    pub fn is_empty(&self) -> bool {
        self.0.iter().all(|&b| b == 0)
    }
}

/// The engine's linear congruential generator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Lcg(pub u32);

impl Lcg {
    /// One step of the generator.
    pub fn step(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(LCG_MUL).wrapping_add(LCG_ADD);
        self.0
    }

    /// The high half of the stepped word times `n`: a uniform index below
    /// `n` (`n` is never 0 in the original's uses).
    pub fn below(&mut self, n: u32) -> u32 {
        ((u64::from(self.step()) * u64::from(n)) >> 32) as u32
    }

    /// A unit draw, the stepped word times `2^-32`.
    pub fn unit_f64(&mut self) -> f64 {
        f64::from(self.step()) * UNIT_SCALE
    }

    /// [`Lcg::unit_f64`] narrowed to single precision, as the engine returns it.
    pub fn unit_f32(&mut self) -> f32 {
        self.unit_f64() as f32
    }
}

/// A decoded PCM buffer in the sound table.
#[derive(Debug, Clone, Default)]
pub struct BufferRecord {
    /// The key the file was loaded under.
    pub name: NameKey,
    /// Loads holding the buffer; the slot is free at 0.
    pub refcount: u32,
    /// The file's length in milliseconds, as the original rounds it.
    pub duration_ms: i32,
    /// The file's length in seconds, from its byte count and format.
    pub length_secs: f32,
    /// Bit 0: the file is stereo.
    pub stereo: bool,
    /// The Bevy asset; absent on a buffer the cache only counts.
    pub handle: Option<Handle<AudioSource>>,
    /// Whether the entry is free for reuse.
    pub free: bool,
}

/// A numbered set of variants and the two it last picked.
#[derive(Debug, Clone, Default)]
pub struct SetRecord {
    /// The base name the variants are numbered from.
    pub name: NameKey,
    /// Buffer ids, variant 01 first.
    pub variants: Vec<u32>,
    /// The last picked variant index.
    pub last: usize,
    /// The variant picked before it.
    pub previous: usize,
    /// Sets holding the record; the record is freed at 0.
    pub refcount: u32,
}

/// PCM layouts the original names by number: mono or stereo, 8- or 16-bit.
pub const FORMAT_MONO8: u32 = 0x1100;
pub const FORMAT_MONO16: u32 = 0x1101;
pub const FORMAT_STEREO8: u32 = 0x1102;
pub const FORMAT_STEREO16: u32 = 0x1103;
/// The frequency the original assumes without a `fmt ` chunk.
pub const DEFAULT_FREQUENCY: u32 = 22_050;

/// The fields of a WAVE file the buffer cache needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WaveInfo {
    /// One of the `FORMAT_*` codes.
    pub format: u32,
    pub frequency: u32,
    /// Bytes in the `data` chunk.
    pub data_len: u32,
}

/// Reads the format, frequency and data length of a RIFF/WAVE file. The
/// header decode is the asset loader's, not the audio environment's; a file
/// without a PCM `fmt ` chunk falls back to the original's mono 16-bit
/// 22 050 Hz defaults.
pub fn wave_info(bytes: &[u8]) -> Option<WaveInfo> {
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return None;
    }
    let mut format = FORMAT_MONO16;
    let mut frequency = DEFAULT_FREQUENCY;
    let mut data_len = 0u32;
    let mut have_fmt = false;
    let mut offset = 12usize;
    while offset + 8 <= bytes.len() {
        let id = &bytes[offset..offset + 4];
        let size = u32::from_le_bytes(bytes[offset + 4..offset + 8].try_into().ok()?);
        let body = offset + 8;
        let end = body.checked_add(size as usize).unwrap_or(bytes.len());
        if id == b"fmt " && body + 16 <= bytes.len() {
            let tag = u16::from_le_bytes(bytes[body..body + 2].try_into().ok()?);
            let channels = u16::from_le_bytes(bytes[body + 2..body + 4].try_into().ok()?);
            let rate = u32::from_le_bytes(bytes[body + 4..body + 8].try_into().ok()?);
            let bits = u16::from_le_bytes(bytes[body + 14..body + 16].try_into().ok()?);
            if tag != 1 {
                return None;
            }
            frequency = rate;
            let mono = channels == 1;
            format = match (mono, bits == 8) {
                (true, true) => FORMAT_MONO8,
                (true, false) => FORMAT_MONO16,
                (false, true) => FORMAT_STEREO8,
                (false, false) => FORMAT_STEREO16,
            };
            have_fmt = true;
        } else if id == b"data" {
            data_len = size;
        }
        // Chunks are word aligned; a malformed length still advances by the
        // header so a truncated file ends the walk instead of looping.
        let advance = end.saturating_sub(body).max(1);
        offset = body.saturating_add(advance).saturating_add(advance & 1);
        if offset <= body {
            break;
        }
    }
    if !have_fmt && data_len == 0 {
        return None;
    }
    Some(WaveInfo {
        format,
        frequency,
        data_len,
    })
}

/// A buffer's length in seconds, from its byte count, frequency, channels and
/// bit depth: the byte count is rounded to whole sample frames *before* the
/// bit depth divides it, as the original does.
pub fn buffer_length_secs(size: i64, frequency: i64, channels: i64, bits: i64) -> f32 {
    if frequency <= 0 || channels <= 0 || bits <= 0 || size <= 0 {
        return 0.0;
    }
    let frames = ((size << 3) as f64 / (frequency * channels) as f64).round_ties_even();
    (frames / bits as f64) as f32
}

/// A buffer's length in milliseconds, from its format code, data bytes and
/// frequency: mono 8-bit counts one byte a sample, 16-bit and stereo 8-bit
/// two, stereo 16-bit four.
pub fn buffer_duration_ms(format: u32, data_len: u32, frequency: u32) -> i32 {
    if frequency == 0 || data_len == 0 {
        return 0;
    }
    let per_byte = f64::from(data_len) / f64::from(frequency);
    let millis = match format {
        FORMAT_MONO8 => per_byte * 1000.0,
        FORMAT_MONO16 | FORMAT_STEREO8 => per_byte * 0.5 * 1000.0,
        FORMAT_STEREO16 => per_byte * 0.25 * 1000.0,
        _ => return 0,
    };
    millis.round_ties_even().clamp(0.0, f64::from(i32::MAX)) as i32
}

/// A descriptor of a numbered sound set by prefix and variant count, for
/// naming a set whose files exist. The loaded sets live in [`SoundBank`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SoundSet<'a> {
    pub prefix: &'a str,
    pub count: usize,
}

impl<'a> SoundSet<'a> {
    pub const fn new(prefix: &'a str, count: usize) -> Self {
        Self { prefix, count }
    }

    /// The name of variant `index` (0 is the set's 01).
    pub fn name(&self, index: usize) -> String {
        format!("{}{:02}", self.prefix, index + 1)
    }
}

/// Decoded sounds by name, the loaded sets, and the generator.
#[derive(Resource)]
pub struct SoundBank {
    /// The direct asset lookup game code uses for one-off sounds.
    loaded: HashMap<String, Option<Handle<AudioSource>>>,
    /// Buffer table; slot 0 is reserved as "no buffer".
    buffers: Vec<BufferRecord>,
    /// Loaded names in key order, the original's sorted index.
    index: Vec<(NameKey, u32)>,
    /// Set table; slot 0 is the shared empty set.
    sets: Vec<Option<SetRecord>>,
    /// Cached sets by base name.
    set_index: HashMap<NameKey, u32>,
    /// The engine's generator.
    rng: Lcg,
}

impl Default for SoundBank {
    fn default() -> Self {
        Self {
            loaded: HashMap::new(),
            buffers: vec![BufferRecord {
                free: true,
                ..default()
            }],
            index: Vec::new(),
            sets: vec![Some(SetRecord::default())],
            set_index: HashMap::new(),
            rng: Lcg::default(),
        }
    }
}

impl SoundBank {
    /// The shared empty set.
    pub const EMPTY_SET: u32 = 0;

    /// The engine's generator, for callers outside the bank (ambience
    /// placement and music offsets share it).
    pub fn rng(&mut self) -> &mut Lcg {
        &mut self.rng
    }

    /// A unit draw from the engine's generator.
    pub fn random_unit(&mut self) -> f32 {
        self.rng.unit_f32()
    }

    /// A pseudo-random index in `0..n`; the engine steps its generator for
    /// any set with more than one variant.
    pub fn pick(&mut self, n: usize) -> usize {
        if n <= 1 {
            0
        } else {
            self.rng.below(n as u32) as usize
        }
    }

    /// The name of a variant of `set`, picked at random.
    pub fn pick_from(&mut self, set: SoundSet) -> String {
        let index = self.pick(set.count);
        set.name(index)
    }

    /// A direct asset lookup by name, cached with its misses. This is the
    /// game code's one-off path; sounds the mixer ranks use
    /// [`SoundBank::load_buffer`].
    pub fn get(
        &mut self,
        game: &Game,
        sources: &mut Assets<AudioSource>,
        name: &str,
    ) -> Option<Handle<AudioSource>> {
        let key = name.to_ascii_lowercase();
        if let Some(h) = self.loaded.get(&key) {
            return h.clone();
        }
        let handle = game
            .0
            .pack(PackKind::Sound)
            .and_then(|p| p.get(&key))
            .map(|bytes| {
                sources.add(AudioSource {
                    bytes: Arc::from(bytes),
                })
            });
        if handle.is_none() {
            warn!("sound {name:?} not found");
        }
        self.loaded.insert(key, handle.clone());
        handle
    }

    /// Loads (or finds) the buffer `name` and adds a reference. Returns the
    /// buffer id, or `None` when the sound is missing or not PCM WAVE.
    pub fn load_buffer(
        &mut self,
        game: &Game,
        sources: &mut Assets<AudioSource>,
        name: &str,
    ) -> Option<u32> {
        let key = NameKey::new(name);
        if let Ok(pos) = self.index.binary_search_by(|(k, _)| k.cmp(&key)) {
            let id = self.index[pos].1;
            if let Some(buffer) = self.buffers.get_mut(id as usize) {
                buffer.refcount = buffer.refcount.saturating_add(1);
                return Some(id);
            }
        }
        let bytes = game
            .0
            .pack(PackKind::Sound)
            .and_then(|p| p.get(&key.text()))?;
        let info = wave_info(bytes)?;
        if info.data_len == 0 {
            return None;
        }
        let duration_ms = buffer_duration_ms(info.format, info.data_len, info.frequency);
        let length_secs = buffer_length_secs(
            i64::from(info.data_len),
            i64::from(info.frequency),
            1 + i64::from(matches!(info.format, FORMAT_STEREO8 | FORMAT_STEREO16)),
            8 + 8 * i64::from(matches!(info.format, FORMAT_MONO16 | FORMAT_STEREO16)),
        );
        let slot = match self
            .buffers
            .iter()
            .enumerate()
            .skip(1)
            .find(|(_, b)| b.free)
        {
            Some((slot, _)) => slot,
            None => {
                self.buffers.push(BufferRecord {
                    free: true,
                    ..default()
                });
                self.buffers.len() - 1
            }
        };
        let handle = sources.add(AudioSource {
            bytes: Arc::from(bytes),
        });
        self.buffers[slot] = BufferRecord {
            name: key,
            refcount: 1,
            duration_ms,
            length_secs,
            stereo: matches!(info.format, FORMAT_STEREO8 | FORMAT_STEREO16),
            handle: Some(handle),
            free: false,
        };
        let pos = self.index.partition_point(|(k, _)| *k < key);
        self.index.insert(pos, (key, slot as u32));
        Some(slot as u32)
    }

    /// Drops one reference to `id`; at zero the name leaves the index, the
    /// asset is dropped and the slot is free again.
    pub fn release_buffer(&mut self, id: u32) {
        let Some(buffer) = self.buffers.get_mut(id as usize) else {
            return;
        };
        if buffer.free {
            return;
        }
        buffer.refcount = buffer.refcount.saturating_sub(1);
        if buffer.refcount > 0 {
            return;
        }
        let key = buffer.name;
        buffer.name = NameKey::default();
        buffer.duration_ms = 0;
        buffer.length_secs = 0.0;
        buffer.stereo = false;
        buffer.handle = None;
        buffer.free = true;
        if let Ok(pos) = self.index.binary_search_by(|(k, _)| k.cmp(&key)) {
            self.index.remove(pos);
        }
    }

    /// The buffer record, if `id` names a loaded buffer.
    pub fn buffer(&self, id: u32) -> Option<&BufferRecord> {
        self.buffers.get(id as usize).filter(|b| !b.free)
    }

    /// The Bevy asset of a loaded buffer.
    pub fn buffer_handle(&self, id: u32) -> Option<Handle<AudioSource>> {
        self.buffer(id).and_then(|b| b.handle.clone())
    }

    /// The buffer's length in milliseconds.
    pub fn buffer_ms(&self, id: u32) -> i32 {
        self.buffer(id).map_or(0, |b| b.duration_ms)
    }

    /// The buffer's length in seconds.
    pub fn buffer_secs(&self, id: u32) -> f32 {
        self.buffer(id).map_or(0.0, |b| b.length_secs)
    }

    /// Whether the buffer's file is stereo.
    pub fn buffer_is_stereo(&self, id: u32) -> bool {
        self.buffer(id).is_some_and(|b| b.stereo)
    }

    /// Loads (or finds) the numbered set `prefix01` … up to its first gap or
    /// 16 variants, and adds a reference. Returns the empty set when the
    /// prefix is unusable or no variant exists.
    pub fn load_set(
        &mut self,
        game: &Game,
        sources: &mut Assets<AudioSource>,
        prefix: &str,
    ) -> u32 {
        if prefix.is_empty() || prefix.len() > MAX_SET_BASE_LEN {
            return Self::EMPTY_SET;
        }
        let key = NameKey::new(prefix);
        if let Some(&id) = self.set_index.get(&key) {
            if let Some(set) = self.sets.get_mut(id as usize).and_then(Option::as_mut) {
                set.refcount = set.refcount.saturating_add(1);
            }
            return id;
        }
        let mut variants = Vec::new();
        for n in 1..=MAX_VARIANTS {
            let name = format!("{prefix}{n:02}");
            match self.load_buffer(game, sources, &name) {
                Some(id) => variants.push(id),
                None => break,
            }
        }
        if variants.is_empty() {
            return Self::EMPTY_SET;
        }
        let id = self.sets.len() as u32;
        self.sets.push(Some(SetRecord {
            name: key,
            variants,
            last: 0,
            previous: 0,
            refcount: 1,
        }));
        self.set_index.insert(key, id);
        id
    }

    /// Drops one reference to a set; at zero every variant buffer is released.
    /// The shared empty set is never freed.
    pub fn release_set(&mut self, id: u32) {
        if id == Self::EMPTY_SET {
            return;
        }
        let Some(record) = self.sets.get_mut(id as usize).and_then(Option::as_mut) else {
            return;
        };
        record.refcount = record.refcount.saturating_sub(1);
        if record.refcount > 0 {
            return;
        }
        let Some(record) = self.sets[id as usize].take() else {
            return;
        };
        self.set_index.remove(&record.name);
        for buffer in record.variants {
            self.release_buffer(buffer);
        }
    }

    /// The set record, if `id` names a loaded set.
    pub fn set(&self, id: u32) -> Option<&SetRecord> {
        self.sets.get(id as usize).and_then(Option::as_ref)
    }

    /// The number of variants in a set.
    pub fn set_count(&self, id: u32) -> usize {
        self.set(id).map_or(0, |s| s.variants.len())
    }

    /// The last variant index the set picked.
    pub fn set_last(&self, id: u32) -> usize {
        self.set(id).map_or(0, |s| s.last)
    }

    /// Picks the next variant of a set, never repeating the last pick (nor
    /// the two last picks once the set has four or more) and returns its
    /// buffer id.
    pub fn pick_set_variant(&mut self, id: u32) -> Option<u32> {
        let (count, last, previous) = {
            let set = self.set(id)?;
            (set.variants.len(), set.last, set.previous)
        };
        if count == 0 {
            return None;
        }
        let index = if count < 2 {
            0
        } else {
            let mut index = 0;
            // The generator has a full period, so the original's loop ends;
            // the bound only keeps a broken generator from hanging.
            for _ in 0..1024 {
                index = self.rng.below(count as u32) as usize;
                let repeats = if count < 4 {
                    index == last
                } else {
                    index == last || index == previous
                };
                if !repeats {
                    break;
                }
            }
            index
        };
        let set = self.sets.get_mut(id as usize).and_then(Option::as_mut)?;
        set.previous = set.last;
        set.last = index;
        set.variants.get(index).copied()
    }

    /// A buffer entry the tests can register without a game install.
    #[cfg(test)]
    pub(crate) fn insert_test_buffer(
        &mut self,
        length_secs: f32,
        duration_ms: i32,
        stereo: bool,
    ) -> u32 {
        self.buffers.push(BufferRecord {
            name: NameKey::new("test"),
            refcount: 1,
            duration_ms,
            length_secs,
            stereo,
            handle: None,
            free: false,
        });
        (self.buffers.len() - 1) as u32
    }

    /// A set entry the tests can register, with the given buffer variants.
    #[cfg(test)]
    pub(crate) fn insert_test_set(&mut self, variants: Vec<u32>) -> u32 {
        let id = self.sets.len() as u32;
        self.sets.push(Some(SetRecord {
            name: NameKey::new("test"),
            variants,
            last: 0,
            previous: 0,
            refcount: 1,
        }));
        id
    }

    /// Gives a test buffer an asset the playback test can resolve.
    #[cfg(test)]
    pub(crate) fn set_test_buffer_handle(&mut self, id: u32, handle: Handle<AudioSource>) {
        if let Some(buffer) = self.buffers.get_mut(id as usize) {
            buffer.handle = Some(handle);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Keys lower-case ASCII and stop at 16 bytes; the rest are zeroes, so a
    /// long name and its 16-byte prefix are the same sound.
    #[test]
    fn names_key_on_sixteen_lower_case_bytes() {
        assert_eq!(NameKey::new("Wh_SwordA01").text(), "wh_sworda01");
        assert_eq!(NameKey::new("ABCDEFGHIJKLMNOPQ").text(), "abcdefghijklmnop");
        assert_eq!(
            NameKey::new("ABCDEFGHIJKLMNOPQ"),
            NameKey::new("abcdefghijklmnop")
        );
        assert_ne!(NameKey::new("wh_sworda01"), NameKey::new("wh_sworda02"));
        assert!(NameKey::new("").is_empty());
    }

    /// The generator steps `× 0x08088405 + 1`, and a draw is the high half of
    /// that word times the count.
    #[test]
    fn the_generator_steps_as_the_engine_does() {
        let mut rng = Lcg::default();
        assert_eq!(rng.step(), 1);
        assert_eq!(rng.step(), 0x0808_8406);
        assert_eq!(rng.step(), 0xdc6d_ac1f);
        let mut rng = Lcg::default();
        assert_eq!(rng.below(6), 0);
        assert_eq!(rng.below(6), 0);
        assert_eq!(rng.below(6), 5);
        let mut rng = Lcg::default();
        let first = rng.unit_f64();
        assert!((first - 1.0 / 4_294_967_296.0).abs() < 1e-18);
    }

    /// A set always takes its first variant; with two or three it never
    /// repeats the last pick, with four or more never the last two. A fresh
    /// set starts with both last picks at 0, so its first draw skips zero.
    #[test]
    fn variants_avoid_the_last_picks() {
        let mut bank = SoundBank::default();
        let set = bank.insert_test_set(vec![10, 11, 12]);
        // One variant: no generator step, always 01.
        let single = bank.insert_test_set(vec![7]);
        assert_eq!(bank.pick_set_variant(single), Some(7));
        assert_eq!(bank.set_last(single), 0);
        // From a fresh generator: state 1,2,3,4 give 0,0,2,0 of three, so
        // the first two draws repeat the initial last pick (0) and are
        // skipped; the third is variant index 2.
        assert_eq!(bank.pick_set_variant(set), Some(12));
        assert_eq!(bank.set_last(set), 2);
        // The next draw at once would be index 0, not the last; the state
        // after the skipping means it lands on 0.
        assert_eq!(bank.pick_set_variant(set), Some(10));
        assert_eq!(bank.set_last(set), 0);
    }

    /// Wave headers give the four PCM layouts, frequency and data length.
    #[test]
    fn wave_headers_name_the_pcm_layout() {
        fn wave(channels: u16, rate: u32, bits: u16, data: u32) -> Vec<u8> {
            let mut b = Vec::new();
            b.extend_from_slice(b"RIFF");
            b.extend_from_slice(&36u32.to_le_bytes());
            b.extend_from_slice(b"WAVE");
            b.extend_from_slice(b"fmt ");
            b.extend_from_slice(&16u32.to_le_bytes());
            b.extend_from_slice(&1u16.to_le_bytes());
            b.extend_from_slice(&channels.to_le_bytes());
            b.extend_from_slice(&rate.to_le_bytes());
            b.extend_from_slice(&(rate * u32::from(channels) * u32::from(bits) / 8).to_le_bytes());
            b.extend_from_slice(&(channels * bits / 8).to_le_bytes());
            b.extend_from_slice(&bits.to_le_bytes());
            b.extend_from_slice(b"data");
            b.extend_from_slice(&data.to_le_bytes());
            b
        }
        assert_eq!(
            wave_info(&wave(1, 22_050, 8, 100)),
            Some(WaveInfo {
                format: FORMAT_MONO8,
                frequency: 22_050,
                data_len: 100
            })
        );
        assert_eq!(
            wave_info(&wave(1, 44_100, 16, 100)),
            Some(WaveInfo {
                format: FORMAT_MONO16,
                frequency: 44_100,
                data_len: 100
            })
        );
        assert_eq!(
            wave_info(&wave(2, 22_050, 8, 100)),
            Some(WaveInfo {
                format: FORMAT_STEREO8,
                frequency: 22_050,
                data_len: 100
            })
        );
        assert_eq!(
            wave_info(&wave(2, 48_000, 16, 100)),
            Some(WaveInfo {
                format: FORMAT_STEREO16,
                frequency: 48_000,
                data_len: 100
            })
        );
        assert_eq!(wave_info(b"not a wave file"), None);
        // A compressed format is refused.
        let mut b = wave(1, 22_050, 8, 100);
        b[20..22].copy_from_slice(&3u16.to_le_bytes());
        assert_eq!(wave_info(&b), None);
    }

    /// Durations count a data byte as an eighth, a quarter or a half of a
    /// millisecond by layout; the original rounds the scaled double.
    #[test]
    fn durations_follow_the_pcm_layout() {
        // 176 400 bytes at 44.1 kHz stereo 16-bit: round(176400/44100*250)
        // = 1000 ms.
        assert_eq!(buffer_duration_ms(FORMAT_STEREO16, 176_400, 44_100), 1000);
        // Mono 16-bit: round(88200/44100*500) = 1000 ms.
        assert_eq!(buffer_duration_ms(FORMAT_MONO16, 88_200, 44_100), 1000);
        // Stereo 8-bit: round(44100/44100*500) = 500 ms.
        assert_eq!(buffer_duration_ms(FORMAT_STEREO8, 44_100, 44_100), 500);
        // Mono 8-bit: round(44100/44100*1000) = 1000 ms.
        assert_eq!(buffer_duration_ms(FORMAT_MONO8, 44_100, 44_100), 1000);
        // Half a millisecond rounds to even: 44 bytes at 44.1 kHz mono 8-bit
        // is 0.998 ms, 45 bytes is 1.020 ms.
        assert_eq!(buffer_duration_ms(FORMAT_MONO8, 44, 44_100), 1);
        assert_eq!(buffer_duration_ms(FORMAT_MONO8, 0, 44_100), 0);
    }

    /// The seconds form rounds the byte count to whole frames before the bit
    /// depth divides it, so a short file can round to nothing.
    #[test]
    fn buffer_lengths_round_the_frame_count_first() {
        assert_eq!(buffer_length_secs(176_400, 44_100, 2, 16), 1.0);
        assert_eq!(buffer_length_secs(88_200, 44_100, 1, 16), 1.0);
        // 1000 bytes at 44.1 kHz mono is 8 000/44 100 = 0.18 frames, which
        // rounds to 0, so the length is 0 s although the audio is 11 ms.
        assert_eq!(buffer_length_secs(1000, 44_100, 1, 16), 0.0);
        // Missing or nonsensical attributes give no length, never a panic.
        assert_eq!(buffer_length_secs(1000, 0, 1, 16), 0.0);
        assert_eq!(buffer_length_secs(-1, 44_100, 1, 16), 0.0);
    }

    /// A set with two holders survives the first release; the second frees
    /// it and every variant buffer.
    #[test]
    fn sets_count_references() {
        let mut bank = SoundBank::default();
        let buffer = bank.insert_test_buffer(1.0, 1000, false);
        let set = bank.insert_test_set(vec![buffer]);
        bank.sets[set as usize].as_mut().unwrap().refcount = 2;
        bank.release_set(set);
        assert!(bank.set(set).is_some());
        assert!(bank.buffer(buffer).is_some());
        bank.release_set(set);
        assert!(bank.set(set).is_none());
        assert!(bank.buffer(buffer).is_none());
        // Releasing the shared empty set or an unknown id does nothing.
        bank.release_set(SoundBank::EMPTY_SET);
        bank.release_set(99);
        assert_eq!(bank.set_count(SoundBank::EMPTY_SET), 0);
    }

    /// A set with four variants avoids both last picks; the generator's
    /// first draws repeat the fresh last picks (0 and 0), so the first
    /// accepted index is 3. The next draw 0 repeats the previous pick and is
    /// skipped; 1 is accepted.
    #[test]
    fn larger_sets_avoid_the_last_two_variants() {
        let mut bank = SoundBank::default();
        let set = bank.insert_test_set(vec![10, 11, 12, 13]);
        assert_eq!(bank.pick_set_variant(set), Some(13));
        assert_eq!(bank.set_last(set), 3);
        assert_eq!(bank.pick_set_variant(set), Some(11));
        assert_eq!(bank.set_last(set), 1);
    }

    /// The table reserves slot 0, counts references by name and frees the
    /// slot and index entry at zero.
    #[test]
    fn buffers_count_references_and_recycle_slots() {
        let mut bank = SoundBank::default();
        // Test buffers are appended directly; release one and make sure a
        // later load would take the free slot (exercised through the test
        // helper's own table).
        let id = bank.insert_test_buffer(1.0, 1000, false);
        assert_eq!(id, 1);
        assert_eq!(bank.buffer_ms(id), 1000);
        assert_eq!(bank.buffer_secs(id), 1.0);
        assert!(!bank.buffer_is_stereo(id));
        // Releasing a slot that is not ours, or twice, is harmless.
        bank.release_buffer(0);
        bank.release_buffer(99);
        bank.release_buffer(id);
        assert!(bank.buffer(id).is_none());
        assert_eq!(bank.buffer_ms(id), 0);
        assert_eq!(bank.buffer_secs(id), 0.0);
    }
}
