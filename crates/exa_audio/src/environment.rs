//! The virtual-voice mixer: 512 voices ranked by audibility onto 64 real
//! sources every frame.
//!
//! A voice carries a gain, a pitch, a position and a buffer. Each update the
//! mixer starts, stops and fades voices, works out how audible each active
//! one is, sorts them high to low, binds the top ones to the free source
//! slots (evicting their previous holders) and stops every source left unused.
//! Music and area ambience are voices too, so their fades go through the same
//! gain rules.
//!
//! The original works in centimetres; gameplay in Exarust works in metres.
//! The rules keep the original's centimetre constants and convert only the
//! positions at the distance test, so the thresholds and curve stay the
//! original's.

use bevy::math::Vec3;
use bevy::prelude::*;

use crate::SoundBank;
use crate::bank::Lcg;

/// Virtual voices in the pool.
pub const VOICE_CAPACITY: usize = 512;
/// Real sources a frame can bind.
pub const SOURCE_CAPACITY: usize = 64;
/// The shared stand-in voice the original hands the music and ambience
/// callers when the pool is full. Writes to it are harmless: it is not one
/// of the pool's voices, is never ranked, and never binds a source.
pub const DUMMY_VOICE: u32 = u32::MAX;
/// Flags a one-shot voice starts with: allocated, auto-free, restart.
pub const DEFAULT_ONE_SHOT_FLAGS: u32 = 0x1104;
/// Audibility of a voice that is not playing.
pub const AUDIBLE_NONE: f32 = -100.0;
/// The smallest audible gain; the original's `Single` epsilon.
pub const SILENCE: f32 = f32::from_bits(0x33ff_ffd7);
/// Squared distance, in cm², beyond which distance lowers a voice's rank:
/// 30 m.
pub const FAR_RADIUS_CM2: f32 = 9_000_000.0;
/// Squared distance, in cm², of the original's unreachable near branch: 4 m.
pub const NEAR_RADIUS_CM2: f32 = 160_000.0;
/// The rolloff's reference distance, in cm: 4 m.
pub const ROLLOFF_REFERENCE_CM: f32 = 400.0;
/// The rolloff's index, 1 in the original.
pub const ROLLOFF_FACTOR: f32 = 1.0;
/// Every source's maximum distance, in cm: 30 m. With the linear clamped
/// distance model a source is silent from here on.
pub const MAX_DISTANCE_CM: f32 = 3000.0;
/// The most a source is heard at after distance, the OpenAL default the
/// original never changes.
pub const HEARD_GAIN_LIMIT: f32 = 1.0;
/// The loudest gain a source is given.
pub const SOURCE_GAIN_LIMIT: f32 = 4.0;
/// The global pitch factor the source pitch is multiplied by.
pub const PITCH_FACTOR: f32 = 1.0;
/// How far along its up vector the listener sits from its anchor, in cm.
pub const LISTENER_PROJECTION_CM: f32 = 300.0;
/// The same projection in metres.
pub const LISTENER_PROJECTION_M: f32 = LISTENER_PROJECTION_CM / 100.0;
/// Centimetres in a metre.
pub const CM_PER_M: f32 = 100.0;

/// Voice flag bits.
pub mod voice_flags {
    /// Playing, or wants a source.
    pub const ACTIVE: u32 = 0x1;
    /// Paused with the game.
    pub const PAUSED: u32 = 0x2;
    /// A restart of the buffer is requested.
    pub const RESTART: u32 = 0x4;
    /// A stop is requested.
    pub const STOP: u32 = 0x8;
    /// Looping.
    pub const LOOPING: u32 = 0x10;
    /// No distance attenuation: the minimum gain is forced to the voice gain.
    pub const NO_ATTENUATION: u32 = 0x20;
    /// The position is relative to the listener anchor.
    pub const LISTENER_RELATIVE: u32 = 0x40;
    /// Free the voice when it ends, stops or loses its source.
    pub const AUTO_FREE: u32 = 0x100;
    /// Fading out; the gain falls by one per second until it stops.
    pub const FADING: u32 = 0x200;
    /// The voice owns a reference to its buffer.
    pub const OWNS_BUFFER: u32 = 0x400;
    /// Allocated from the pool.
    pub const ALLOCATED: u32 = 0x1000;
    /// The interface/music bus: not placed in the world, no reverb send,
    /// keeps playing while the game is paused.
    pub const INTERFACE: u32 = 0x4000;
}

/// Where the listener is and which way it faces.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Listener {
    /// The anchor both the listener and relative sounds use. In the original
    /// game it is the player's body plus 50 cm up.
    pub anchor: Vec3,
    /// The facing direction, the first half of the orientation.
    pub forward: Vec3,
    /// The up direction, the second half of the orientation; the listener
    /// sits 3 m along it.
    pub up: Vec3,
}

impl Default for Listener {
    fn default() -> Self {
        Self {
            anchor: Vec3::ZERO,
            forward: Vec3::ZERO,
            up: Vec3::ZERO,
        }
    }
}

impl Listener {
    /// The listener's own position: the anchor plus 3 m along the up vector
    /// (the vector after the facing in the orientation the original sends).
    pub fn position(&self) -> Vec3 {
        self.anchor + self.up * LISTENER_PROJECTION_M
    }
}

/// The two bus gains a frame mixes through.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BusGains {
    /// The master gain, never written in the original.
    pub master: f32,
    /// The screen-transition fade.
    pub transition: f32,
    /// The window-focus fade, stepped once an update.
    pub focus: f32,
    /// The world fade applied to the world bus only.
    pub world_fade: f32,
}

impl Default for BusGains {
    fn default() -> Self {
        Self {
            master: 1.0,
            transition: 1.0,
            focus: 1.0,
            world_fade: 1.0,
        }
    }
}

impl BusGains {
    /// The interface/music bus: master × transition × focus.
    pub fn interface(&self) -> f32 {
        self.master * self.transition * self.focus
    }

    /// The world bus: the interface bus × the world fade.
    pub fn world(&self) -> f32 {
        self.interface() * self.world_fade
    }
}

/// One virtual voice.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Voice {
    pub flags: u32,
    /// The ranking key; −100 when not playing.
    pub audibility: f32,
    /// The bound source slot, or −1.
    pub source: i32,
    /// The buffer id, or 0.
    pub buffer: u32,
    pub gain: f32,
    pub pitch: f32,
    /// End time in milliseconds for a non-looping voice.
    pub end_ms: i64,
    /// Start offset in seconds, consumed when playback starts.
    pub offset: f32,
    pub position: Vec3,
    /// The handle attached to this voice.
    pub owner: Option<u32>,
}

/// A handle a game object holds to one voice, the original's virtual sound
/// source: either bound to a voice or free.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VoiceHandle(u32);

impl VoiceHandle {
    /// The "no sound" handle.
    pub const NONE: Self = Self(u32::MAX);

    /// The handle's raw slot, for tests.
    pub fn index(self) -> u32 {
        self.0
    }
}

/// A source's playback parameters, after the bus gain and clamp.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SourcePlay {
    pub slot: u8,
    pub buffer: u32,
    pub gain: f32,
    /// The pitch after the global factor.
    pub pitch: f32,
    /// Start offset in seconds, applied once.
    pub offset: f32,
    pub looping: bool,
    /// Start attached but not playing.
    pub paused: bool,
    /// Absolute position in metres.
    pub position: Vec3,
    /// The minimum gain when the voice defeats distance attenuation.
    pub min_gain: Option<f32>,
    /// Whether the voice is on the listener-relative interface bus.
    pub relative: bool,
    /// Whether the voice would get the reverb send.
    pub reverb: bool,
    /// A restart of a bound source rather than a fresh binding. OpenAL
    /// refuses a new buffer on a source that is playing or paused, so such a
    /// source keeps its buffer: a paused one resumes, a playing one starts
    /// again from the top.
    pub restart: bool,
}

/// A bound source's per-frame properties.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SourceUpdate {
    pub slot: u8,
    pub gain: f32,
    pub pitch: f32,
    pub position: Vec3,
    pub min_gain: Option<f32>,
}

/// What the mixer asks the backend to do to a source this frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SourceCommand {
    Stop {
        slot: u8,
    },
    Pause {
        slot: u8,
    },
    Resume {
        slot: u8,
    },
    /// Attach a buffer and start (or start paused).
    Play(SourcePlay),
    /// Change a playing source's properties.
    Refresh(SourceUpdate),
}

/// One frame's inputs besides the voices and the bank.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UpdateInput {
    /// Real time in milliseconds, the music and fade clock.
    pub now_ms: u32,
    /// Game time in milliseconds, the ambience clock (it stops when paused).
    pub game_ms: u32,
    /// The game is paused.
    pub paused: bool,
    /// The window is focused.
    pub focused: bool,
    /// Which bound sources have finished.
    pub source_stopped: [bool; SOURCE_CAPACITY],
}

impl Default for UpdateInput {
    fn default() -> Self {
        Self {
            now_ms: 0,
            game_ms: 0,
            paused: false,
            focused: true,
            source_stopped: [false; SOURCE_CAPACITY],
        }
    }
}

/// The audio environment: the voice pool, the listener, the buses, and the
/// music and ambience that play through it.
#[derive(Resource, Debug)]
pub struct AudioEnvironment {
    /// Object flags; creation sets bit 0 and bit 4, initialisation adds bits
    /// 0 and 1, and one-shots need both bits 0 and 1.
    pub object_flags: u32,
    pub listener: Listener,
    pub buses: BusGains,
    /// Flags a one-shot voice is created with.
    pub default_flags: u32,
    /// The voices, indexed by id.
    voices: Vec<Voice>,
    /// The shared voice handed out when the pool is full.
    dummy: Voice,
    /// The sort order: voice ids, ranked by audibility.
    order: Vec<u32>,
    /// One past the highest voice ever allocated.
    high_water: usize,
    /// The lowest id a free voice may be found from.
    hint: usize,
    /// Which sources the top voices used this update.
    used: Vec<bool>,
    /// Which voice each source is bound to.
    source_owner: Vec<Option<u32>>,
    /// The handle table.
    handles: Vec<Option<u32>>,
    /// The previous update's real time, for the fade step.
    prev_ms: u32,
    /// The original's "already ran this frame" flag.
    pub ran_this_frame: bool,
    /// The commands of the last update.
    pub(crate) commands: Vec<SourceCommand>,
    /// The music crossfader's state.
    pub music: crate::music::MusicState,
    /// The area ambience's state.
    pub ambience: crate::ambience::AmbienceState,
    /// The global EFX reverb.
    pub reverb: crate::reverb::Reverb,
    /// A stand-in for the runtime library's own random stream, which the
    /// ambience placement draws its distance from (the engine generator
    /// serves every other draw). The real stream's generator and seed are
    /// outside the evidence; this one is the engine generator from 0.
    pub rtl_random: Lcg,
}

impl Default for AudioEnvironment {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioEnvironment {
    /// A live environment: the constructor's flags plus initialisation, a
    /// 512-voice pool, 64 sources and the identity order.
    pub fn new() -> Self {
        let voices = vec![Voice::default(); VOICE_CAPACITY];
        Self {
            object_flags: 0x13,
            listener: Listener::default(),
            buses: BusGains::default(),
            default_flags: DEFAULT_ONE_SHOT_FLAGS,
            voices,
            dummy: Voice {
                source: -1,
                ..Voice::default()
            },
            order: (0..VOICE_CAPACITY as u32).collect(),
            high_water: 0,
            hint: 0,
            used: vec![false; SOURCE_CAPACITY],
            source_owner: vec![None; SOURCE_CAPACITY],
            handles: Vec::new(),
            prev_ms: 0,
            ran_this_frame: false,
            commands: Vec::new(),
            music: crate::music::MusicState::default(),
            ambience: crate::ambience::AmbienceState::default(),
            reverb: crate::reverb::Reverb::default(),
            rtl_random: Lcg::default(),
        }
    }

    /// The silent stand-in environment: no voice or one-shot plays.
    pub fn silent() -> Self {
        Self {
            object_flags: 0,
            ..Self::new()
        }
    }

    /// Whether one-shots and attached sounds may start.
    pub fn is_live(&self) -> bool {
        self.object_flags & 3 == 3
    }

    /// The voices, for reading a rank or a flag. The shared dummy voice is
    /// readable by its id, with its source at −1.
    pub fn voice(&self, id: u32) -> Option<&Voice> {
        if id == DUMMY_VOICE {
            return Some(&self.dummy);
        }
        self.voices.get(id as usize)
    }

    /// A voice, for a caller that keeps one directly.
    pub fn voice_mut(&mut self, id: u32) -> Option<&mut Voice> {
        if id == DUMMY_VOICE {
            return Some(&mut self.dummy);
        }
        self.voices.get_mut(id as usize)
    }

    /// A copy of the current voice order.
    pub fn order(&self) -> &[u32] {
        &self.order
    }

    /// The highest allocated voice plus one.
    pub fn high_water(&self) -> usize {
        self.high_water
    }

    /// Queues a stop of the source a voice is bound to, without unbinding it.
    pub(crate) fn stop_bound_source(&mut self, id: u32) {
        let Some(voice) = self.voices.get(id as usize) else {
            return;
        };
        if voice.source >= 0 && (voice.source as usize) < SOURCE_CAPACITY {
            self.commands.push(SourceCommand::Stop {
                slot: voice.source as u8,
            });
        }
    }

    /// The source slots the last update used.
    pub fn used_sources(&self) -> &[bool] {
        &self.used
    }

    /// The commands of the last update.
    pub fn commands(&self) -> &[SourceCommand] {
        &self.commands
    }

    /// Queues a command for the playback test.
    #[cfg(test)]
    pub(crate) fn push_test_command(&mut self, command: SourceCommand) {
        self.commands.push(command);
    }

    /// Drops the queued test commands.
    #[cfg(test)]
    pub(crate) fn clear_test_commands(&mut self) {
        self.commands.clear();
    }

    /// Allocates a handle, free until attached to a voice.
    pub fn new_handle(&mut self) -> VoiceHandle {
        let id = self.handles.len() as u32;
        self.handles.push(None);
        VoiceHandle(id)
    }

    /// The voice a handle is bound to, if any.
    pub fn handle_voice(&self, handle: VoiceHandle) -> Option<u32> {
        self.handles.get(handle.0 as usize).copied().flatten()
    }

    /// Whether a handle is bound to a live voice.
    pub fn handle_is_bound(&self, handle: VoiceHandle) -> bool {
        self.handles
            .get(handle.0 as usize)
            .is_some_and(|voice| voice.is_some())
    }

    /// Allocates a voice from the free hint upward. Returns `None` when full.
    pub fn allocate(&mut self) -> Option<u32> {
        let capacity = self.voices.len();
        if self.hint >= capacity {
            return None;
        }
        let mut index = self.hint;
        while index < capacity {
            if self.voices[index].flags == 0 {
                self.voices[index].flags = voice_flags::ALLOCATED;
                self.voices[index].source = -1;
                if index >= self.high_water {
                    self.high_water = index + 1;
                }
                return Some(index as u32);
            }
            index += 1;
        }
        None
    }

    /// Allocates a voice for the music and ambience callers: like
    /// [`Self::allocate`], but when the pool is full it hands back the
    /// shared dummy voice, whose writes are harmless, as the original does.
    pub fn allocate_or_dummy(&mut self) -> u32 {
        self.allocate().unwrap_or(DUMMY_VOICE)
    }

    /// Lets go of a voice a caller holds by option: a pool voice is freed and
    /// the shared dummy is simply dropped. Returns the cleared holder.
    pub fn release_voice(&mut self, bank: &mut SoundBank, slot: Option<u32>) -> Option<u32> {
        if let Some(id) = slot
            && id != DUMMY_VOICE
        {
            self.free_voice(bank, id);
        }
        None
    }

    /// Frees a voice now: detaches the handle, stops and unbinds its source,
    /// releases an owned buffer, clears its flags and lowers the free hint.
    /// The shared dummy voice is not in the pool, so freeing it does nothing.
    pub fn free_voice(&mut self, bank: &mut SoundBank, id: u32) {
        if id == DUMMY_VOICE {
            return;
        }
        let Some(index) = usize::try_from(id)
            .ok()
            .filter(|index| *index < self.voices.len())
        else {
            return;
        };
        let mut voice = self.voices[index];
        if let Some(handle) = voice.owner.take()
            && let Some(slot) = self.handles.get_mut(handle as usize)
        {
            *slot = None;
        }
        if voice.source >= 0 {
            let slot = voice.source as usize;
            if slot < SOURCE_CAPACITY {
                self.commands.push(SourceCommand::Stop { slot: slot as u8 });
                self.source_owner[slot] = None;
            }
            voice.source = -1;
        }
        voice.audibility = AUDIBLE_NONE;
        if voice.flags & voice_flags::OWNS_BUFFER != 0 {
            bank.release_buffer(voice.buffer);
            voice.buffer = 0;
        }
        voice.flags = 0;
        if index < self.hint {
            self.hint = index;
        }
        self.voices[index] = voice;
    }

    /// Frees a voice once it has finished: a playing non-looping voice is
    /// marked auto-free and plays on (an attached handle stays attached until
    /// the voice is freed), anything else is freed now.
    pub fn finish_voice(&mut self, bank: &mut SoundBank, id: u32) {
        let Some(voice) = self.voices.get(id as usize).copied() else {
            return;
        };
        if voice.flags & (voice_flags::ACTIVE | voice_flags::LOOPING) == voice_flags::ACTIVE {
            self.voices[id as usize].flags |= voice_flags::AUTO_FREE;
        } else {
            self.free_voice(bank, id);
        }
    }

    /// Starts a one-shot from a buffer at a position.
    pub fn play_buffer(
        &mut self,
        buffer: u32,
        position: Vec3,
        gain: f32,
        pitch: f32,
        extra_flags: u32,
    ) -> Option<u32> {
        if !self.is_live() {
            return None;
        }
        let id = self.allocate()?;
        let voice = &mut self.voices[id as usize];
        voice.flags = self.default_flags | extra_flags;
        voice.buffer = buffer;
        voice.position = position;
        voice.gain = gain;
        voice.pitch = pitch;
        Some(id)
    }

    /// Starts a one-shot from a random variant of a set.
    pub fn play_set(
        &mut self,
        bank: &mut SoundBank,
        set: u32,
        position: Vec3,
        gain: f32,
        pitch: f32,
        extra_flags: u32,
    ) -> Option<u32> {
        if !self.is_live() {
            return None;
        }
        let buffer = bank.pick_set_variant(set)?;
        let id = self.allocate()?;
        let voice = &mut self.voices[id as usize];
        voice.flags = self.default_flags | extra_flags;
        voice.buffer = buffer;
        voice.position = position;
        voice.gain = gain;
        voice.pitch = pitch;
        Some(id)
    }

    /// Starts a variant of a set attached to a caller-held handle.
    #[allow(clippy::too_many_arguments)]
    pub fn play_set_attached(
        &mut self,
        bank: &mut SoundBank,
        set: u32,
        handle: VoiceHandle,
        position: Vec3,
        gain: f32,
        pitch: f32,
    ) -> bool {
        if handle == VoiceHandle::NONE || !self.is_live() || bank.set_count(set) == 0 {
            return false;
        }
        let Some(id) = self.allocate() else {
            return false;
        };
        if let Some(old) = self.handles.get(handle.0 as usize).copied().flatten()
            && let Some(voice) = self.voices.get_mut(old as usize)
        {
            voice.owner = None;
        }
        if let Some(slot) = self.handles.get_mut(handle.0 as usize) {
            *slot = Some(id);
        }
        let Some(buffer) = bank.pick_set_variant(set) else {
            return false;
        };
        let voice = &mut self.voices[id as usize];
        voice.flags = self.default_flags;
        voice.owner = Some(handle.0);
        voice.buffer = buffer;
        voice.position = position;
        voice.gain = gain;
        voice.pitch = pitch;
        true
    }

    /// Starts a looping variant of a set attached to a caller-held handle:
    /// the held items' and the body's loops. Like [`Self::play_set_attached`]
    /// but with the looping flag. Returns false when the handle is unset,
    /// the environment is silent or the set is empty.
    #[allow(clippy::too_many_arguments)]
    pub fn play_loop_attached(
        &mut self,
        bank: &mut SoundBank,
        set: u32,
        handle: VoiceHandle,
        position: Vec3,
        gain: f32,
        pitch: f32,
    ) -> bool {
        if handle == VoiceHandle::NONE || !self.is_live() || bank.set_count(set) == 0 {
            return false;
        }
        let Some(id) = self.allocate() else {
            return false;
        };
        if let Some(old) = self.handles.get(handle.0 as usize).copied().flatten()
            && let Some(voice) = self.voices.get_mut(old as usize)
        {
            voice.owner = None;
        }
        if let Some(slot) = self.handles.get_mut(handle.0 as usize) {
            *slot = Some(id);
        }
        let Some(buffer) = bank.pick_set_variant(set) else {
            return false;
        };
        let voice = &mut self.voices[id as usize];
        voice.flags = self.default_flags | voice_flags::LOOPING;
        voice.owner = Some(handle.0);
        voice.buffer = buffer;
        voice.position = position;
        voice.gain = gain;
        voice.pitch = pitch;
        true
    }

    /// Starts a buffer on a caller-held handle: like [`Self::play_buffer`],
    /// but the voice is bound to `handle` (a voice it held before is let go)
    /// so the caller can move, re-level and free it later. Returns false when
    /// the handle is unset, the environment is silent or the pool is full.
    #[allow(clippy::too_many_arguments)]
    pub fn play_buffer_attached(
        &mut self,
        buffer: u32,
        handle: VoiceHandle,
        position: Vec3,
        gain: f32,
        pitch: f32,
        extra_flags: u32,
    ) -> bool {
        if handle == VoiceHandle::NONE || !self.is_live() {
            return false;
        }
        let Some(id) = self.allocate() else {
            return false;
        };
        if let Some(old) = self.handles.get(handle.0 as usize).copied().flatten()
            && let Some(voice) = self.voices.get_mut(old as usize)
        {
            voice.owner = None;
        }
        if let Some(slot) = self.handles.get_mut(handle.0 as usize) {
            *slot = Some(id);
        }
        let voice = &mut self.voices[id as usize];
        voice.flags = self.default_flags | extra_flags;
        voice.owner = Some(handle.0);
        voice.buffer = buffer;
        voice.position = position;
        voice.gain = gain;
        voice.pitch = pitch;
        true
    }

    /// Frees a handle's voice: the original's source release. Leaves the
    /// handle unbound.
    pub fn free_handle(&mut self, bank: &mut SoundBank, handle: VoiceHandle) {
        if let Some(id) = self.handles.get(handle.0 as usize).copied().flatten() {
            self.free_voice(bank, id);
        }
    }

    /// Restarts a voice on a random variant of a set, keeping the caller's
    /// record. The original assigns the restart flags rather than adding them,
    /// and, unlike the one-shots, does not ask whether the environment is live.
    pub fn retrigger_set(
        &mut self,
        bank: &mut SoundBank,
        id: u32,
        set: u32,
        position: Vec3,
        gain: f32,
        pitch: f32,
    ) {
        let Some(buffer) = bank.pick_set_variant(set) else {
            return;
        };
        let Some(voice) = self.voices.get_mut(id as usize) else {
            return;
        };
        voice.flags = voice_flags::ALLOCATED | voice_flags::RESTART;
        if voice.source >= 0 && (voice.source as usize) < SOURCE_CAPACITY {
            self.commands.push(SourceCommand::Stop {
                slot: voice.source as u8,
            });
        }
        voice.buffer = buffer;
        voice.position = position;
        voice.gain = gain;
        voice.pitch = pitch;
    }

    /// Restarts a voice on a given buffer.
    pub fn retrigger_buffer(
        &mut self,
        id: u32,
        buffer: u32,
        position: Vec3,
        gain: f32,
        pitch: f32,
    ) {
        let Some(voice) = self.voices.get_mut(id as usize) else {
            return;
        };
        voice.flags |= voice_flags::ALLOCATED | voice_flags::RESTART;
        if buffer != voice.buffer {
            voice.buffer = buffer;
        }
        voice.position = position;
        voice.gain = gain;
        voice.pitch = pitch;
    }

    /// Sets a voice's start offset in seconds.
    pub fn set_start_offset(&mut self, id: u32, offset: f32) {
        if let Some(voice) = self.voices.get_mut(id as usize) {
            voice.offset = offset;
        }
    }

    /// Runs one update: ambience scheduling, the music crossfader, the focus
    /// and bus gains, then the voice scan, sort and source assignment. The
    /// returned commands are for the backend.
    pub fn update(&mut self, bank: &mut SoundBank, input: &UpdateInput) -> &[SourceCommand] {
        self.commands.clear();
        self.ran_this_frame = true;
        if self.ambience.installed {
            self.schedule_ambience(bank, input.game_ms);
        }
        self.advance_music(bank, input.now_ms);
        // The steps are double-precision constants added to the stored single.
        if input.focused {
            self.buses.focus = (f64::from(self.buses.focus) + 0.01) as f32;
            if 1.0 < self.buses.focus {
                self.buses.focus = 1.0;
            }
        } else {
            self.buses.focus = (f64::from(self.buses.focus) - 0.02) as f32;
            if self.buses.focus < 0.0 {
                self.buses.focus = 0.0;
            }
        }
        let active = self.scan(bank, input.now_ms, &input.source_stopped);
        let high_water = self.high_water;
        if high_water > 0 {
            let (voices, order) = (&self.voices, &mut self.order);
            sort_desc(&mut order[..high_water], voices, 0, high_water - 1);
        }
        self.used.fill(false);
        let mut last = active;
        if SOURCE_CAPACITY <= last {
            last = SOURCE_CAPACITY - 1;
        }
        if input.paused {
            for index in 0..=last {
                let Some(&id) = self.order.get(index) else {
                    break;
                };
                let Some(voice) = self.voices.get(id as usize).copied() else {
                    continue;
                };
                if voice.flags & voice_flags::ACTIVE == 0 || voice.source < 0 {
                    continue;
                }
                if voice.flags & voice_flags::INTERFACE == 0 {
                    if (voice.source as usize) < SOURCE_CAPACITY {
                        self.commands.push(SourceCommand::Pause {
                            slot: voice.source as u8,
                        });
                        self.voices[id as usize].flags |= voice_flags::PAUSED;
                    }
                } else {
                    self.refresh(bank, id);
                }
                if let Some(used) = self.used.get_mut(voice.source as usize) {
                    *used = true;
                }
            }
        } else {
            for index in 0..=last {
                let Some(&id) = self.order.get(index) else {
                    break;
                };
                let Some(voice) = self.voices.get(id as usize).copied() else {
                    continue;
                };
                if voice.flags & voice_flags::ACTIVE == 0 || voice.source < 0 {
                    continue;
                }
                if voice.flags & voice_flags::PAUSED != 0 {
                    if (voice.source as usize) < SOURCE_CAPACITY {
                        self.commands.push(SourceCommand::Resume {
                            slot: voice.source as u8,
                        });
                    }
                    self.voices[id as usize].flags &= !voice_flags::PAUSED;
                }
                self.refresh(bank, id);
                if let Some(used) = self.used.get_mut(voice.source as usize) {
                    *used = true;
                }
            }
        }
        let mut slot = 0;
        for index in 0..=last {
            let Some(&id) = self.order.get(index) else {
                break;
            };
            let Some(voice) = self.voices.get(id as usize).copied() else {
                continue;
            };
            if voice.flags & voice_flags::ACTIVE == 0
                || voice.source >= 0
                || voice.flags & voice_flags::STOP != 0
            {
                continue;
            }
            while slot < SOURCE_CAPACITY && self.used[slot] {
                slot += 1;
            }
            if slot < SOURCE_CAPACITY {
                self.bind(bank, id, slot);
                self.used[slot] = true;
                slot += 1;
            }
        }
        if high_water > 0 && last + 1 < high_water {
            for index in last + 1..high_water {
                let Some(&id) = self.order.get(index) else {
                    continue;
                };
                let Some(voice) = self.voices.get(id as usize).copied() else {
                    continue;
                };
                if voice.flags & voice_flags::ALLOCATED == 0 {
                    continue;
                }
                if voice.flags & voice_flags::AUTO_FREE == 0 {
                    let mut voice = voice;
                    voice.flags &= !voice_flags::ACTIVE;
                    if voice.flags & voice_flags::LOOPING != 0 {
                        voice.flags |= voice_flags::RESTART;
                    }
                    self.voices[id as usize] = voice;
                } else if voice.flags & voice_flags::ACTIVE != 0 {
                    self.free_voice(bank, id);
                }
            }
        }
        for slot in 0..SOURCE_CAPACITY {
            if !self.used[slot] {
                self.commands.push(SourceCommand::Stop { slot: slot as u8 });
            }
        }
        &self.commands
    }

    /// The original's per-frame scheduler callback: runs the update unless
    /// something already ran it this frame, then clears the flag.
    pub fn scheduler_tick(&mut self, bank: &mut SoundBank, input: &UpdateInput) -> bool {
        let ran = if self.ran_this_frame {
            false
        } else {
            self.update(bank, input);
            true
        };
        self.ran_this_frame = false;
        ran
    }

    /// The voice scan: start, fade, stop and free requests, and the audibility
    /// of every voice. Returns how many are active.
    fn scan(
        &mut self,
        bank: &mut SoundBank,
        now_ms: u32,
        source_stopped: &[bool; SOURCE_CAPACITY],
    ) -> usize {
        // The original subtracts the two 32-bit times as 64-bit values, so a
        // wrap yields a negative interval rather than a small positive one.
        let elapsed =
            (u64::from(now_ms).wrapping_sub(u64::from(self.prev_ms)) as i64) as f64 * 0.001;
        self.prev_ms = now_ms;
        let listener = self.listener.position();
        let mut active = 0;
        for index in 0..self.high_water {
            let mut voice = self.voices[index];
            if voice.flags & voice_flags::RESTART != 0 {
                voice.flags |= voice_flags::ACTIVE;
                voice.flags &= !voice_flags::STOP;
            } else {
                if voice.flags & voice_flags::ACTIVE == 0 {
                    voice.audibility = AUDIBLE_NONE;
                    self.voices[index] = voice;
                    continue;
                }
                if voice.flags & voice_flags::FADING != 0 {
                    let gain = voice.gain - elapsed as f32;
                    voice.gain = gain;
                    if gain < SILENCE {
                        voice.gain = 0.0;
                        voice.flags = (voice.flags & !voice_flags::FADING) | voice_flags::STOP;
                    }
                }
                if voice.flags & voice_flags::STOP == 0 {
                    let stopped = voice.flags & voice_flags::LOOPING == 0
                        && voice.source >= 0
                        && source_stopped
                            .get(voice.source as usize)
                            .copied()
                            .unwrap_or(false);
                    if stopped {
                        if voice.flags & voice_flags::AUTO_FREE != 0 {
                            self.free_voice(bank, index as u32);
                            continue;
                        }
                        if voice.source >= 0 {
                            if (voice.source as usize) < SOURCE_CAPACITY {
                                self.commands.push(SourceCommand::Stop {
                                    slot: voice.source as u8,
                                });
                            }
                            voice.flags &= !voice_flags::ACTIVE;
                        }
                    }
                } else {
                    voice.flags &= !(voice_flags::ACTIVE
                        | voice_flags::RESTART
                        | voice_flags::STOP
                        | voice_flags::LOOPING);
                    if voice.flags & voice_flags::AUTO_FREE == 0 {
                        if voice.source >= 0 && (voice.source as usize) < SOURCE_CAPACITY {
                            self.commands.push(SourceCommand::Stop {
                                slot: voice.source as u8,
                            });
                        }
                    } else {
                        self.free_voice(bank, index as u32);
                        continue;
                    }
                }
            }
            if voice.flags & voice_flags::ACTIVE == 0 {
                voice.audibility = AUDIBLE_NONE;
            } else {
                active += 1;
                voice.audibility = rank(&voice, listener);
            }
            self.voices[index] = voice;
        }
        active
    }

    /// Binds a voice to a source slot, evicting its holder, and gives the
    /// source its buffer, gains, pitch, position, looping and start offset.
    fn bind(&mut self, bank: &mut SoundBank, id: u32, slot: usize) {
        if slot >= SOURCE_CAPACITY {
            // The original parks the voice on source 0 when it is handed a
            // slot outside the source table.
            if let Some(voice) = self.voices.get_mut(id as usize) {
                voice.source = 0;
            }
            return;
        }
        if let Some(old) = self.source_owner[slot].take() {
            // The holder loses the slot first, so freeing it stops nothing.
            let mut auto_free = false;
            if let Some(voice) = self.voices.get_mut(old as usize) {
                voice.source = -1;
                auto_free = voice.flags & voice_flags::AUTO_FREE != 0;
                if !auto_free {
                    voice.flags &= !voice_flags::ACTIVE;
                }
            }
            if auto_free {
                self.free_voice(bank, old);
            }
        }
        let Some(index) = usize::try_from(id)
            .ok()
            .filter(|index| *index < self.voices.len())
        else {
            return;
        };
        let mut voice = self.voices[index];
        voice.source = slot as i32;
        self.source_owner[slot] = Some(id);
        let interface = voice.flags & voice_flags::INTERFACE != 0;
        let bus = if interface {
            self.buses.interface()
        } else {
            self.buses.world()
        };
        let mut gain = voice.gain * bus;
        if SOURCE_GAIN_LIMIT < gain {
            gain = SOURCE_GAIN_LIMIT;
        }
        let pitch = voice.pitch * PITCH_FACTOR;
        let relative = voice.flags & voice_flags::LISTENER_RELATIVE != 0;
        let position = if relative {
            voice.position + self.listener.anchor
        } else {
            voice.position
        };
        let min_gain = (voice.flags & voice_flags::NO_ATTENUATION != 0)
            .then_some(voice.gain * self.buses.interface());
        let looping = voice.flags & voice_flags::LOOPING != 0;
        let paused = voice.flags & voice_flags::PAUSED != 0;
        let offset = if !paused && SILENCE < voice.offset {
            let offset = voice.offset;
            voice.offset = 0.0;
            offset
        } else {
            0.0
        };
        if !looping {
            voice.end_ms = i64::from(self.prev_ms)
                + (bank.buffer_ms(voice.buffer) as f32 / voice.pitch).round_ties_even() as i64;
        }
        if !paused {
            voice.flags |= voice_flags::ACTIVE;
        }
        voice.flags &= !voice_flags::RESTART;
        self.voices[index] = voice;
        self.commands.push(SourceCommand::Stop { slot: slot as u8 });
        self.commands.push(SourceCommand::Play(SourcePlay {
            slot: slot as u8,
            buffer: voice.buffer,
            gain,
            pitch,
            offset,
            looping,
            paused,
            position,
            min_gain,
            relative,
            reverb: !interface && self.reverb.enabled,
            restart: false,
        }));
    }

    /// Refreshes a bound source's gain, pitch and position, and restarts its
    /// buffer and playback when one is requested.
    fn refresh(&mut self, bank: &SoundBank, id: u32) {
        let Some(index) = usize::try_from(id)
            .ok()
            .filter(|index| *index < self.voices.len())
        else {
            return;
        };
        let mut voice = self.voices[index];
        if voice.source < 0 || voice.source as usize >= SOURCE_CAPACITY {
            voice.source = 0;
            self.voices[index] = voice;
            return;
        }
        let slot = voice.source as usize;
        let interface = voice.flags & voice_flags::INTERFACE != 0;
        let bus = if interface {
            self.buses.interface()
        } else {
            self.buses.world()
        };
        let mut gain = voice.gain * bus;
        if SOURCE_GAIN_LIMIT < gain {
            gain = SOURCE_GAIN_LIMIT;
        }
        let pitch = voice.pitch * PITCH_FACTOR;
        let relative = voice.flags & voice_flags::LISTENER_RELATIVE != 0;
        let position = if relative {
            voice.position + self.listener.anchor
        } else {
            voice.position
        };
        let min_gain = (voice.flags & voice_flags::NO_ATTENUATION != 0).then_some(gain);
        self.commands.push(SourceCommand::Refresh(SourceUpdate {
            slot: slot as u8,
            gain,
            pitch,
            position,
            min_gain,
        }));
        if voice.flags & voice_flags::RESTART != 0 {
            let offset = if SILENCE < voice.offset {
                let offset = voice.offset;
                voice.offset = 0.0;
                offset
            } else {
                0.0
            };
            let looping = voice.flags & voice_flags::LOOPING != 0;
            voice.end_ms = i64::from(self.prev_ms)
                + (bank.buffer_ms(voice.buffer) as f32 / voice.pitch).round_ties_even() as i64;
            voice.flags = (voice.flags & !voice_flags::RESTART) | voice_flags::ACTIVE;
            self.commands.push(SourceCommand::Play(SourcePlay {
                slot: slot as u8,
                buffer: voice.buffer,
                gain,
                pitch,
                offset,
                looping,
                paused: false,
                position,
                min_gain,
                relative,
                reverb: !interface && self.reverb.enabled,
                restart: true,
            }));
        }
        self.voices[index] = voice;
    }
}

/// How audible a voice is, from its raw gain and squared distance to the
/// listener. Below 30 m the rank is the gain; from 30 m the original scales
/// it by `4 m / d`, its near branch (4 m) being unreachable as written.
pub fn rank(voice: &Voice, listener: Vec3) -> f32 {
    if voice.flags & voice_flags::ACTIVE == 0 {
        return AUDIBLE_NONE;
    }
    if voice.flags & voice_flags::LISTENER_RELATIVE != 0 {
        return voice.gain;
    }
    let offset = (voice.position - listener) * CM_PER_M;
    let distance2 = offset.x * offset.x + offset.y * offset.y + offset.z * offset.z;
    if FAR_RADIUS_CM2 <= distance2 {
        if distance2 <= NEAR_RADIUS_CM2 {
            voice.gain
        } else {
            (ROLLOFF_REFERENCE_CM
                / ((distance2.sqrt() - ROLLOFF_REFERENCE_CM) * ROLLOFF_FACTOR
                    + ROLLOFF_REFERENCE_CM))
                * voice.gain
        }
    } else {
        voice.gain
    }
}

/// What OpenAL makes of a source's gain at a distance, as the original sets
/// it up: the linear clamped distance model with a 4 m reference, a 30 m
/// maximum and rolloff 1, then the gain clamped between the source's minimum
/// gain and the default maximum of 1. Distances are converted to cm.
pub fn heard_gain(gain: f32, min_gain: Option<f32>, source: Vec3, listener: Vec3) -> f32 {
    let distance = (source - listener).length() * CM_PER_M;
    let clamped = distance.clamp(ROLLOFF_REFERENCE_CM, MAX_DISTANCE_CM);
    let attenuation = 1.0
        - ROLLOFF_FACTOR * (clamped - ROLLOFF_REFERENCE_CM)
            / (MAX_DISTANCE_CM - ROLLOFF_REFERENCE_CM);
    (gain * attenuation.max(0.0))
        .max(min_gain.unwrap_or(0.0))
        .min(HEARD_GAIN_LIMIT)
}

/// The original's Hoare quicksort over the order array, high audibility
/// first. Equal ranks come out in the same order as the original's swaps.
fn sort_desc(order: &mut [u32], voices: &[Voice], first: usize, last: usize) {
    if order.is_empty() || last >= order.len() {
        return;
    }
    let mut first = first;
    loop {
        let mut left = first;
        let Some(pivot_id) = order.get((first + last) / 2) else {
            return;
        };
        let pivot = voices
            .get(*pivot_id as usize)
            .map_or(AUDIBLE_NONE, |voice| voice.audibility);
        let mut right = last;
        loop {
            while left <= last
                && voices
                    .get(order[left] as usize)
                    .is_some_and(|voice| voice.audibility > pivot)
            {
                left += 1;
            }
            while right >= first
                && voices
                    .get(order[right] as usize)
                    .is_some_and(|voice| voice.audibility < pivot)
            {
                if right == 0 {
                    break;
                }
                right -= 1;
            }
            if left <= right {
                order.swap(left, right);
                left += 1;
                if right == 0 {
                    break;
                }
                right -= 1;
            }
            if left > right {
                break;
            }
        }
        if first < right {
            sort_desc(order, voices, first, right);
        }
        first = left;
        if left >= last {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bank::Lcg;

    fn voice(flags: u32, gain: f32, position: Vec3) -> Voice {
        Voice {
            flags,
            gain,
            position,
            ..Default::default()
        }
    }

    /// The listener sits 3 m along its up vector, not its facing; a relative
    /// voice ranks by its own gain and a world voice below 30 m does too.
    #[test]
    fn audibility_uses_gain_inside_thirty_metres() {
        let listener = Listener {
            anchor: Vec3::new(100.0, 50.0, 0.0),
            forward: Vec3::Z,
            up: Vec3::Y,
        };
        assert_eq!(listener.position(), Vec3::new(100.0, 53.0, 0.0));
        let at = voice(voice_flags::ACTIVE, 0.7, Vec3::new(100.0, 50.0, 0.0));
        assert_eq!(rank(&at, listener.position()), 0.7);
        let far = voice(
            voice_flags::ACTIVE | voice_flags::LISTENER_RELATIVE,
            0.25,
            Vec3::ZERO,
        );
        assert_eq!(rank(&far, listener.position()), 0.25);
        let quiet = voice(0, 1.0, Vec3::ZERO);
        assert_eq!(rank(&quiet, listener.position()), AUDIBLE_NONE);
    }

    /// From 30 m the rank is the gain times 4 m over the distance: at 30 m
    /// that is 0.1333, at 200 m 0.02.
    #[test]
    fn audibility_falls_as_four_metres_over_the_distance() {
        let listener = Vec3::ZERO;
        let edge = voice(voice_flags::ACTIVE, 1.0, Vec3::X * 30.0);
        let score = rank(&edge, listener);
        assert!((score - 400.0 / 3000.0).abs() < 1e-6, "{score}");
        let far = voice(voice_flags::ACTIVE, 1.0, Vec3::X * 200.0);
        assert!((rank(&far, listener) - 0.02).abs() < 1e-6);
        // Half of 30 m is still the raw gain.
        let near = voice(voice_flags::ACTIVE, 1.0, Vec3::X * 14.0);
        assert_eq!(rank(&near, listener), 1.0);
    }

    /// Allocation takes the lowest free voice from the hint, raises the high
    /// water mark, and a freed voice is found again from that index.
    #[test]
    fn voices_allocate_from_the_hint_and_recycle() {
        let mut env = AudioEnvironment::new();
        assert_eq!(env.allocate(), Some(0));
        assert_eq!(env.allocate(), Some(1));
        assert_eq!(env.high_water(), 2);
        let one = env.voice(1).copied().unwrap();
        assert_eq!(one.flags, voice_flags::ALLOCATED);
        assert_eq!(one.source, -1);
        let mut bank = SoundBank::default();
        env.free_voice(&mut bank, 1);
        assert_eq!(env.voice(1).unwrap().flags, 0);
        assert_eq!(env.voice(1).unwrap().audibility, AUDIBLE_NONE);
        assert_eq!(env.allocate(), Some(1));
        assert_eq!(env.high_water(), 2);
        // Filling the pool returns no voice rather than a dummy.
        for _ in 0..VOICE_CAPACITY {
            env.allocate();
        }
        assert_eq!(env.allocate(), None);
    }

    /// A buffer started on a handle is bound to it, so the caller finds the
    /// same voice next frame and frees it through the handle.
    #[test]
    fn a_buffer_on_a_handle_stays_bound_until_freed() {
        let mut env = AudioEnvironment::new();
        let mut bank = SoundBank::default();
        assert!(!env.play_buffer_attached(1, VoiceHandle::NONE, Vec3::ZERO, 1.0, 1.0, 0));
        let handle = env.new_handle();
        assert!(env.play_buffer_attached(1, handle, Vec3::X, 0.5, 0.9, voice_flags::LOOPING));
        let id = env.handle_voice(handle).expect("bound");
        let voice = env.voice(id).copied().unwrap();
        assert_eq!(voice.buffer, 1);
        assert_eq!((voice.gain, voice.pitch), (0.5, 0.9));
        assert_ne!(voice.flags & voice_flags::LOOPING, 0);
        env.free_handle(&mut bank, handle);
        assert_eq!(env.handle_voice(handle), None);
        assert_eq!(env.voice(id).unwrap().flags, 0);
    }

    /// A stopped source frees an auto-free voice, deactivates any other and
    /// lets a looping voice keep its slot.
    #[test]
    fn the_scan_stops_and_frees_finished_voices() {
        let mut env = AudioEnvironment::new();
        let mut bank = SoundBank::default();
        let one = env
            .play_buffer(1, Vec3::ZERO, 1.0, 1.0, 0)
            .expect("a voice");
        assert_eq!(one, 0);
        // The first update binds it.
        let mut input = UpdateInput::default();
        env.update(&mut bank, &input);
        assert_eq!(env.voice(0).unwrap().source, 0);
        assert!(
            env.commands()
                .iter()
                .any(|command| matches!(command, SourceCommand::Play(play) if play.slot == 0))
        );
        // Report the source finished: an auto-free voice disappears.
        input.now_ms = 10;
        input.source_stopped[0] = true;
        env.update(&mut bank, &input);
        assert_eq!(env.voice(0).unwrap().flags, 0);
        assert_eq!(env.voice(0).unwrap().source, -1);
    }

    // A pattern for the matches! above; keeps the test readable without
    // binding every field of SourcePlay.
    #[allow(dead_code)]
    struct SourceCommandPlay {
        slot: u8,
    }

    /// A fading voice loses one gain per second of real time and stops at
    /// silence; the pair of updates spans half a second.
    #[test]
    fn fading_voices_lose_gain_per_second() {
        let mut env = AudioEnvironment::new();
        let mut bank = SoundBank::default();
        let id = env
            .play_buffer(1, Vec3::ZERO, 1.0, 1.0, 0)
            .expect("a voice");
        // Keep the voice after it stops, to see the fade reach zero.
        env.voice_mut(id).unwrap().flags =
            (env.voice(id).unwrap().flags | voice_flags::FADING) & !voice_flags::AUTO_FREE;
        let input = UpdateInput::default();
        env.update(&mut bank, &input);
        let input = UpdateInput {
            now_ms: 500,
            ..UpdateInput::default()
        };
        env.update(&mut bank, &input);
        let gain = env.voice(id).unwrap().gain;
        assert!((gain - 0.5).abs() < 1e-6, "{gain}");
        let input = UpdateInput {
            now_ms: 1500,
            ..UpdateInput::default()
        };
        env.update(&mut bank, &input);
        let voice = env.voice(id).unwrap();
        assert_eq!(voice.gain, 0.0);
        assert_eq!(voice.flags & voice_flags::FADING, 0);
        assert_eq!(voice.flags & voice_flags::ACTIVE, 0);
        assert_eq!(voice.flags & voice_flags::STOP, 0);
    }

    /// The buses multiply in order: the interface bus is master × transition
    /// × focus, the world bus takes the world fade too.
    #[test]
    fn buses_multiply_like_the_original() {
        let buses = BusGains {
            master: 0.8,
            transition: 0.5,
            focus: 0.25,
            world_fade: 0.1,
        };
        assert!((buses.interface() - 0.1).abs() < 1e-6);
        assert!((buses.world() - 0.01).abs() < 1e-6);
        // The focus steps by 0.01 a frame up to 1 and 0.02 down to 0.
        let mut env = AudioEnvironment::new();
        env.buses.focus = 0.99;
        env.update(&mut SoundBank::default(), &UpdateInput::default());
        assert_eq!(env.buses.focus, 1.0);
        env.update(
            &mut SoundBank::default(),
            &UpdateInput {
                focused: false,
                ..UpdateInput::default()
            },
        );
        assert!((env.buses.focus - 0.98).abs() < 1e-6);
        env.buses.focus = 0.0;
        env.update(
            &mut SoundBank::default(),
            &UpdateInput {
                focused: false,
                ..UpdateInput::default()
            },
        );
        assert_eq!(env.buses.focus, 0.0);
    }

    /// The order sorts high audibility first with the original's swaps, so a
    /// known set of ranks gives a known permutation.
    #[test]
    fn the_order_sorts_by_audibility() {
        let mut voices = vec![Voice::default(); 5];
        for (i, rank) in [3.0, 9.0, 1.0, 9.0, 5.0].into_iter().enumerate() {
            voices[i].audibility = rank;
        }
        let mut order = vec![0, 1, 2, 3, 4];
        sort_desc(&mut order, &voices, 0, 4);
        let ranks: Vec<f32> = order
            .iter()
            .map(|&i| voices[i as usize].audibility)
            .collect();
        assert_eq!(ranks, vec![9.0, 9.0, 5.0, 3.0, 1.0]);
    }

    /// A binding carries the voice gain times the bus (clamped to 4), the
    /// pitch, an absolute position, the looping flag and the end time.
    #[test]
    fn a_binding_carries_the_source_properties() {
        let mut env = AudioEnvironment::new();
        let mut bank = SoundBank::default();
        let buffer = bank.insert_test_buffer(2.0, 2000, false);
        let id = env
            .play_buffer(buffer, Vec3::new(1.0, 2.0, 3.0), 2.0, 1.5, 0)
            .expect("a voice");
        env.buses.focus = 0.49;
        env.buses.world_fade = 0.5;
        env.reverb.set_enabled(true);
        env.update(&mut bank, &UpdateInput::default());
        let play = env
            .commands()
            .iter()
            .find_map(|command| match command {
                SourceCommand::Play(play) => Some(*play),
                _ => None,
            })
            .expect("a play command");
        assert_eq!(play.slot, 0);
        assert_eq!(play.buffer, buffer);
        // The focus steps up to 0.5 first; the interface bus is then
        // 1.0×1.0×0.5 = 0.5, the world bus 0.25, and gain 2.0 → 0.5.
        assert!((play.gain - 0.5).abs() < 1e-6, "{}", play.gain);
        assert_eq!(play.pitch, 1.5);
        assert_eq!(play.position, Vec3::new(1.0, 2.0, 3.0));
        assert!(!play.looping);
        assert!(!play.relative);
        assert!(play.reverb);
        assert_eq!(env.voice(id).unwrap().end_ms, 1333);
    }

    /// A world voice gets the reverb send only while the reverb is enabled.
    #[test]
    fn world_voices_get_the_send_only_when_reverb_is_on() {
        let mut env = AudioEnvironment::new();
        let mut bank = SoundBank::default();
        let buffer = bank.insert_test_buffer(1.0, 1000, false);
        env.play_buffer(buffer, Vec3::ZERO, 1.0, 1.0, 0)
            .expect("a voice");
        env.update(&mut bank, &UpdateInput::default());
        let play = env
            .commands()
            .iter()
            .find_map(|command| match command {
                SourceCommand::Play(play) => Some(*play),
                _ => None,
            })
            .expect("a play command");
        assert!(!play.reverb);
        env.reverb.set_enabled(true);
        let id = env
            .play_buffer(buffer, Vec3::X * 10.0, 1.0, 1.0, 0)
            .expect("a voice");
        env.update(&mut bank, &UpdateInput::default());
        let play = env
            .commands()
            .iter()
            .find_map(|command| match command {
                SourceCommand::Play(play) if play.slot == 1 => Some(*play),
                _ => None,
            })
            .expect("a play command");
        assert!(play.reverb);
        let _ = id;
    }

    /// The scan only looks at voices up to the high-water mark, as the
    /// original's loop does; an unallocated voice above it keeps its zeroed
    /// rank.
    #[test]
    fn the_scan_stops_at_the_high_water_mark() {
        let mut env = AudioEnvironment::new();
        let mut bank = SoundBank::default();
        env.play_buffer(1, Vec3::ZERO, 1.0, 1.0, 0)
            .expect("a voice");
        env.update(&mut bank, &UpdateInput::default());
        assert_eq!(env.voice(0).unwrap().audibility, 1.0);
        assert_eq!(env.voice(1).unwrap().audibility, 0.0);
        assert_eq!(env.voice(500).unwrap().audibility, 0.0);
        assert_eq!(env.high_water(), 1);
    }

    /// A voice on the interface bus is placed relative to the anchor, gets no
    /// reverb send, and its minimum gain is the gain times the interface bus.
    #[test]
    fn interface_voices_are_relative_and_dry() {
        let mut env = AudioEnvironment::new();
        let mut bank = SoundBank::default();
        let buffer = bank.insert_test_buffer(1.0, 1000, false);
        env.listener.anchor = Vec3::new(10.0, 0.0, 0.0);
        let id = env
            .play_buffer(
                buffer,
                Vec3::new(1.0, 0.0, 0.0),
                1.0,
                1.0,
                voice_flags::INTERFACE
                    | voice_flags::LISTENER_RELATIVE
                    | voice_flags::NO_ATTENUATION
                    | voice_flags::LOOPING,
            )
            .expect("a voice");
        env.update(&mut bank, &UpdateInput::default());
        let play = env
            .commands()
            .iter()
            .find_map(|command| match command {
                SourceCommand::Play(play) => Some(*play),
                _ => None,
            })
            .expect("a play command");
        assert!(play.relative);
        assert!(!play.reverb);
        assert_eq!(play.position, Vec3::new(11.0, 0.0, 0.0));
        assert_eq!(play.min_gain, Some(1.0));
        assert!(play.looping);
        assert_eq!(env.voice(id).unwrap().end_ms, 0);
    }

    /// The top active voices take the free source slots in order; the first
    /// sixty-four fit and the rest stay unbound. A voice that loses its place
    /// is deactivated, or restart-marked when it loops.
    #[test]
    fn the_top_voices_take_the_source_slots() {
        let mut env = AudioEnvironment::new();
        let mut bank = SoundBank::default();
        for _ in 0..(SOURCE_CAPACITY + 2) {
            env.play_buffer(1, Vec3::ZERO, 1.0, 1.0, 0)
                .expect("a voice");
        }
        env.update(&mut bank, &UpdateInput::default());
        assert_eq!(
            env.used_sources().iter().filter(|used| **used).count(),
            SOURCE_CAPACITY
        );
        let bound = (0..(SOURCE_CAPACITY + 2) as u32)
            .filter(|id| env.voice(*id).unwrap().source >= 0)
            .count();
        assert_eq!(bound, SOURCE_CAPACITY);
        // The two voices the ranking put beyond the top are auto-free
        // one-shots, so they are freed rather than just deactivated.
        let freed = (0..(SOURCE_CAPACITY + 2) as u32)
            .filter(|id| env.voice(*id).unwrap().flags == 0)
            .count();
        assert_eq!(freed, 2);
    }

    /// A louder voice that enters the top sixty-four evicts the quietest
    /// voice that held a source, freeing it when it is auto-free.
    #[test]
    fn a_louder_voice_evicts_the_quietest_bound_one() {
        let mut env = AudioEnvironment::new();
        let mut bank = SoundBank::default();
        for i in 0..SOURCE_CAPACITY {
            env.play_buffer(1, Vec3::ZERO, 0.5 + i as f32 * 0.001, 1.0, 0)
                .expect("a voice");
        }
        env.update(&mut bank, &UpdateInput::default());
        let quiet = env.order()[SOURCE_CAPACITY - 1];
        assert_eq!(
            env.voice(quiet).unwrap().source,
            (SOURCE_CAPACITY - 1) as i32
        );
        // A much louder voice ranks first and takes the slot of the voice
        // that fell out of the top.
        let loud = env
            .play_buffer(1, Vec3::ZERO, 2.0, 1.0, 0)
            .expect("a voice");
        env.update(&mut bank, &UpdateInput::default());
        let slot = env.voice(loud).unwrap().source as usize;
        assert_eq!(slot, SOURCE_CAPACITY - 1);
        assert_eq!(env.voice(quiet).unwrap().source, -1);
        assert_eq!(env.voice(quiet).unwrap().flags, 0);
    }

    /// While the game is paused, world voices are paused and refreshed with
    /// the interface voices, and the pause bit is set.
    #[test]
    fn pausing_holds_world_voices_but_refreshes_the_interface() {
        let mut env = AudioEnvironment::new();
        let mut bank = SoundBank::default();
        let buffer = bank.insert_test_buffer(1.0, 1000, false);
        let world = env
            .play_buffer(buffer, Vec3::X * 5.0, 1.0, 1.0, 0)
            .expect("a world voice");
        let interface = env
            .play_buffer(
                buffer,
                Vec3::ZERO,
                1.0,
                1.0,
                voice_flags::INTERFACE | voice_flags::LISTENER_RELATIVE,
            )
            .expect("an interface voice");
        env.update(&mut bank, &UpdateInput::default());
        assert!(env.voice(world).unwrap().source >= 0);
        assert!(env.voice(interface).unwrap().source >= 0);
        env.update(
            &mut bank,
            &UpdateInput {
                paused: true,
                ..UpdateInput::default()
            },
        );
        assert_eq!(
            env.voice(world).unwrap().flags & voice_flags::PAUSED,
            voice_flags::PAUSED
        );
        assert_eq!(env.voice(interface).unwrap().flags & voice_flags::PAUSED, 0);
        assert!(
            env.commands()
                .iter()
                .any(|command| matches!(command, SourceCommand::Pause { .. }))
        );
        assert!(env.commands().iter().any(|command| matches!(
            command,
            SourceCommand::Refresh(update) if update.slot == env.voice(interface).unwrap().source as u8
        )));
    }

    /// Finishing a playing one-shot marks it auto-free and lets it play on,
    /// still attached to its handle until it is freed; finishing a looping
    /// voice frees it now. A voice needs no handle to be finished.
    #[test]
    fn finishing_marks_one_shots_and_frees_loops() {
        let mut env = AudioEnvironment::new();
        let mut bank = SoundBank::default();
        let set = bank.insert_test_set(vec![1]);
        let handle = env.new_handle();
        assert!(env.play_set_attached(&mut bank, set, handle, Vec3::ZERO, 1.0, 1.0));
        let voice = env.handles[handle.index() as usize].unwrap();
        env.voices[voice as usize].flags |= voice_flags::ACTIVE;
        env.finish_voice(&mut bank, voice);
        assert_eq!(
            env.voice(voice).unwrap().flags & voice_flags::AUTO_FREE,
            voice_flags::AUTO_FREE
        );
        assert!(env.handle_is_bound(handle));
        assert_ne!(env.voice(voice).unwrap().flags, 0);
        env.free_voice(&mut bank, voice);
        assert!(!env.handle_is_bound(handle));

        let bare = env.allocate().expect("a voice");
        env.voices[bare as usize].flags |= voice_flags::ACTIVE;
        env.finish_voice(&mut bank, bare);
        assert_ne!(env.voice(bare).unwrap().flags & voice_flags::AUTO_FREE, 0);
        let idle = env.allocate().expect("a voice");
        env.finish_voice(&mut bank, idle);
        assert_eq!(env.voice(idle).unwrap().flags, 0);

        let handle = env.new_handle();
        assert!(env.play_set_attached(&mut bank, set, handle, Vec3::ZERO, 1.0, 1.0));
        let looping = env.handles[handle.index() as usize].unwrap();
        env.voices[looping as usize].flags |= voice_flags::ACTIVE | voice_flags::LOOPING;
        env.finish_voice(&mut bank, looping);
        assert_eq!(env.voice(looping).unwrap().flags, 0);
        assert!(!env.handle_is_bound(handle));
    }

    /// A one-shot is refused by the silent stand-in environment.
    #[test]
    fn the_silent_environment_starts_nothing() {
        let mut env = AudioEnvironment::silent();
        let mut bank = SoundBank::default();
        assert_eq!(env.play_buffer(1, Vec3::ZERO, 1.0, 1.0, 0), None);
        let set = bank.insert_test_set(vec![1]);
        assert_eq!(env.play_set(&mut bank, set, Vec3::ZERO, 1.0, 1.0, 0), None);
        assert!(!env.is_live());
    }

    /// The scheduler callback runs the update only when nothing else did
    /// this frame, and clears the flag afterwards.
    #[test]
    fn the_scheduler_runs_the_update_once() {
        let mut env = AudioEnvironment::new();
        let mut bank = SoundBank::default();
        let input = UpdateInput::default();
        env.update(&mut bank, &input);
        assert!(!env.scheduler_tick(&mut bank, &input));
        assert!(!env.ran_this_frame);
        assert!(env.scheduler_tick(&mut bank, &input));
        assert!(!env.ran_this_frame);
    }

    /// A handle is bound while it points at a voice and returns to the
    /// sentinel when that voice is freed.
    #[test]
    fn handles_track_their_voice() {
        let mut env = AudioEnvironment::new();
        let mut bank = SoundBank::default();
        let set = bank.insert_test_set(vec![1, 2]);
        let handle = env.new_handle();
        assert!(!env.handle_is_bound(handle));
        assert!(env.play_set_attached(&mut bank, set, handle, Vec3::ZERO, 1.0, 1.0));
        assert!(env.handle_is_bound(handle));
        let voice = env.handles[handle.index() as usize].unwrap();
        assert_eq!(env.voice(voice).unwrap().owner, Some(handle.index()));
        env.free_voice(&mut bank, voice);
        assert!(!env.handle_is_bound(handle));
        assert_eq!(env.voice(voice).unwrap().owner, None);
    }

    /// Retriggering a set assigns the restart flags (dropping the interface
    /// and auto-free bits) and changes the buffer, position, gain and pitch.
    #[test]
    fn retriggering_replaces_the_variant_and_flags() {
        let mut env = AudioEnvironment::new();
        let mut bank = SoundBank::default();
        let set = bank.insert_test_set(vec![4, 5]);
        let id = env
            .play_buffer(9, Vec3::ZERO, 1.0, 1.0, voice_flags::AUTO_FREE)
            .expect("a voice");
        env.retrigger_set(&mut bank, id, set, Vec3::X, 0.5, 1.25);
        let voice = env.voice(id).unwrap();
        assert_eq!(voice.flags, voice_flags::ALLOCATED | voice_flags::RESTART);
        // The fresh generator draws 0, 0, then 1 of two variants.
        assert_eq!(voice.buffer, 5);
        assert_eq!(voice.position, Vec3::X);
        assert_eq!(voice.gain, 0.5);
        assert_eq!(voice.pitch, 1.25);
        env.retrigger_buffer(id, 9, Vec3::Y, 0.75, 1.5);
        let voice = env.voice(id).unwrap();
        assert_eq!(voice.flags, voice_flags::ALLOCATED | voice_flags::RESTART);
        assert_eq!(voice.buffer, 9);
        assert_eq!(voice.position, Vec3::Y);
    }

    /// Retriggering does not ask whether the environment is live, unlike the
    /// one-shots.
    #[test]
    fn retriggering_works_in_the_silent_environment() {
        let mut env = AudioEnvironment::silent();
        let mut bank = SoundBank::default();
        let set = bank.insert_test_set(vec![4]);
        env.retrigger_set(&mut bank, 0, set, Vec3::X, 0.5, 1.0);
        assert_eq!(
            env.voice(0).unwrap().flags,
            voice_flags::ALLOCATED | voice_flags::RESTART
        );
        assert_eq!(env.voice(0).unwrap().buffer, 4);
    }

    /// The distance model is linear between 4 m and 30 m, silent beyond, and
    /// the heard gain is held between the minimum gain and 1.
    #[test]
    fn sources_are_heard_on_a_linear_four_to_thirty_metre_slope() {
        let at = Vec3::ZERO;
        assert_eq!(heard_gain(0.8, None, Vec3::X * 2.0, at), 0.8);
        assert_eq!(heard_gain(0.8, None, Vec3::X * 4.0, at), 0.8);
        // 17 m: 1 − 13/26 = 0.5.
        assert!((heard_gain(0.8, None, Vec3::X * 17.0, at) - 0.4).abs() < 1e-6);
        assert_eq!(heard_gain(0.8, None, Vec3::X * 30.0, at), 0.0);
        assert_eq!(heard_gain(0.8, None, Vec3::X * 90.0, at), 0.0);
        // A voice that defeats attenuation is held at its minimum gain.
        assert_eq!(heard_gain(0.3, Some(0.3), Vec3::X * 90.0, at), 0.3);
        // Above 1 the default maximum gain holds it.
        assert_eq!(heard_gain(2.5, None, Vec3::X, at), 1.0);
    }

    /// The window focus steps in double precision, so 0.99 + 0.01 lands just
    /// past 1 and is held there, and 0.5 − 0.02 is the single nearest 0.48.
    #[test]
    fn the_focus_steps_in_double_precision() {
        let mut env = AudioEnvironment::new();
        let mut bank = SoundBank::default();
        env.buses.focus = 0.5;
        env.update(
            &mut bank,
            &UpdateInput {
                focused: false,
                ..UpdateInput::default()
            },
        );
        assert_eq!(env.buses.focus, (f64::from(0.5f32) - 0.02) as f32);
        env.buses.focus = 0.3;
        env.update(&mut bank, &UpdateInput::default());
        assert_eq!(env.buses.focus, (f64::from(0.3f32) + 0.01) as f32);
    }

    /// The engine's generator drives the ambience and music draws: its first
    /// two words are 1 and 0x08088406.
    #[test]
    fn the_environment_shares_the_engine_generator() {
        let mut env = AudioEnvironment::new();
        assert_eq!(env.rtl_random, Lcg::default());
        assert_eq!(env.rtl_random.step(), 1);
    }

    /// A full pool hands the music and ambience callers the shared dummy
    /// voice: its writes are harmless, it never binds a source and freeing it
    /// only lets the caller's holder go. The nil-returning allocation still
    /// reports no voice.
    #[test]
    fn a_full_pool_hands_out_the_shared_dummy_voice() {
        let mut env = AudioEnvironment::new();
        let mut bank = SoundBank::default();
        for _ in 0..VOICE_CAPACITY {
            env.allocate().expect("a pool voice");
        }
        assert_eq!(env.allocate(), None);
        assert_eq!(env.allocate_or_dummy(), DUMMY_VOICE);
        let dummy = *env.voice(DUMMY_VOICE).expect("the dummy");
        assert_eq!(dummy.flags, 0);
        assert_eq!(dummy.source, -1);
        // Writes land in the stand-in and are never heard.
        let id = env.allocate_or_dummy();
        env.voice_mut(id).unwrap().gain = 5.0;
        env.voice_mut(id).unwrap().buffer = 7;
        assert_eq!(env.voice(id).unwrap().gain, 5.0);
        assert!(!env.used_sources().iter().any(|used| *used));
        env.free_voice(&mut bank, id);
        assert_eq!(env.voice(id).unwrap().gain, 5.0);
        // The holder is dropped without touching the pool.
        assert_eq!(env.release_voice(&mut bank, Some(id)), None);
        assert_eq!(env.high_water(), VOICE_CAPACITY);
    }
}
