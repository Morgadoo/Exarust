# `exa_audio`

Audio: PCM sound playback, positional 3D sound, reverb environments, footstep/impact/voice sets, music.

**Phase:** Phase 5 (see [`docs/PLAN.md`](../../docs/PLAN.md)). Status: the
sound bank and its numbered sets, the original's virtual-voice mixer, music
crossfades, area ambience and the reverb parameters are in. EFX and the
original's OpenAL distance model are not applied by the backend (see
[Gaps](#gaps)).

## Modules

| module | what it holds |
|---|---|
| `bank` | the name-keyed buffer cache (`Sound.rpk` WAV files), numbered sets, the variant picker and the engine generator |
| `environment` | the 512-voice pool, listener and bus gains, audibility ranking, the quicksort, source binding and the per-update state machine |
| `music` | the two-track crossfader (A/B), its four states and the level's track choice; a story level's track, volume, ambience and preset by the bytes of its name, and whether entering it changes the music |
| `ambience` | an area's looping bed and its scheduled one-shots |
| `reverb` | the global EAX reverb parameter block and the level's decay offset |
| `hits`, `footsteps` | the game's first two users of the mixer |
| `playback` | one Bevy audio entity per real source slot, driven by the mixer's commands |

## Rules recovered from the original

### Buffer cache and sound sets (`bank`)

- A sound's identity is the first 16 bytes of its lower-cased name, zero
  padded. Longer names collide with their 16-byte prefix.
- The buffer table reserves slot 0 as "no buffer". A load searches slot 1
  onward for a free slot, growing the table by one when none is free, sets the
  reference count to 1 and inserts the name into a key-sorted index.
- A hit in the index adds a reference. A release subtracts; at zero the name
  leaves the index, the slot and its asset are dropped and the slot is reused.
- Durations come from the WAV header: mono 8-bit counts a data byte as
  `1/frequency` seconds, mono 16-bit and stereo 8-bit as `0.5/frequency`, and
  stereo 16-bit as `0.25/frequency`, rounded to whole milliseconds. The
  seconds form used for music offsets rounds the frame count **before** the
  bit depth divides it, as the original does (a short file can round to 0 s).
- A set is built from `base01`, `base02`, … stopping at the first missing
  file or at 16 variants. A base name must be 1 to 14 characters. Sets keep a
  reference count and are shared by base name; releasing the last holder
  releases every variant buffer.
- A variant pick steps the engine's generator (`state ← state·0x08088405 +
  1`, index = high half of `state · count`). One variant always picks 0
  without stepping; two or three never repeat the last pick; four or more
  never repeat either of the last two. A fresh set has both last picks at 0,
  so its first pick skips variant 01.

### Mixer (`environment`)

- 512 voices. Allocation scans upward from a free hint, marks a voice
  allocated, gives it source −1 and raises the high-water mark. A free lowers
  the hint. Fields other than the flags and source are not cleared on reuse.
- An update runs, in this order: the ambience one-shot scheduler, the music
  state machine, the window-focus step, the voice scan, the audibility sort,
  source assignment, the voices beyond the top, then stopping every source
  left unused.
- Focus falls by 0.02 per update while the window is unfocused and rises by
  0.01 to 1. The interface bus is master × transition × focus; the world bus
  is the interface bus × the world fade.
- The listener sits at the anchor plus 3 m along its facing. The original is
  in centimetres; gameplay is in metres, so the distance test scales the
  offset back to centimetres before using the original constants.
- Audibility is the raw gain below 30 m; from 30 m it is `gain · 4 m / d`.
  The original's "inside 4 m" branch sits inside the "beyond 30 m" test and
  can never run; it is kept in the same place.
- The sort is the original's Hoare quicksort, high first, with its exact swap
  order for equal ranks.
- The active count is used as the last top index (not count − 1), so one
  extra voice is looked at; it is inactive by construction unless the cap
  applies. At 64 or more actives the last index is 63, giving exactly 64.
- Binding evicts the slot's previous voice: an auto-free one is freed, any
  other is deactivated. It then attaches the buffer, sends world voices to
  the reverb when enabled, sets the gain (voice gain × bus, clamped to 4),
  the pitch, the position (absolute, or anchor-relative), the minimum gain
  when the voice defeats distance attenuation, the looping flag and the end
  time, and starts it unless paused. The start offset is consumed once.
- A refresh updates the gain, pitch and position every frame and re-attaches
  the buffer when a restart was requested.
- Finished non-looping voices free themselves when auto-free. A voice beyond
  the top loses its active bit; a looping one keeps a restart request so it
  resumes when it ranks high again. Auto-free ones are freed.
- Music and ambience voices are ordinary interface-bus voices (relative to
  the listener, unattenuated, no reverb send, looping).

### Music (`music`)

- Two tracks: A is playing, B is incoming or the overlay. All fades last
  3000 ms on the real-time clock.
- Playing a track on A now swaps its buffer and restarts it at the given
  volume, remembering the name and volume.
- A crossfade forces music on: B rises linearly from 0 while A falls as
  `(1 − t)²`; at the end A is freed (only if it had a source) and its buffer
  released, and B becomes A.
- An overlay fade-in starts B at a random point in the first 80 % of its
  length, ducks A as `(1 − t)²`, and pauses A at the end.
- A fade-out resumes A (`(1 − t)²` → linear) and frees B at the end.
- Starting either fade during the other mirrors its progress, so the gains
  continue without a jump.
- The Music setting stops A below 0.01 volume threshold rules: only a volume
  above 0.01 acts; stopping A keeps its name and buffer, and turning music on
  restarts it from them.
- Observed quirks kept: the promoted track's volume is not written into A's
  nominal volume (the original writes it to a global nothing reads), B's
  buffer is not released when an overlay ends, and stops keep references.

### Ambience (`ambience`)

- An area `X` has a bed `EnvX01` at `0.15 × volume`, pitch 1, looped on the
  interface bus, started at a random point in the first 0.8 s, and a one-shot
  set `AmbX01…16`.
- Setting an area releases the old bed buffer only when it changes and the
  old set after loading the new one; the bed voice is reused.
- The one-shot scheduler runs on the game clock. The first shot is due
  15–45 s after an area is installed. A mono shot is placed 10–20 m from the
  listener anchor in a random horizontal direction, at gain 0.02–0.05 for
  the first two variants and 0.03–0.10 for the rest, pitch 0.9–1.1, and is
  rescheduled 5–15 s later. A stereo shot stays on the listener, at gain
  0.075–0.15 and pitch 0.8–1.0, and is rescheduled 15–45 s later.
- Only the outdoor arenas' known daylight case is wired: `NtrA` at volume 1
  (a still, dry day; the weather and time-of-day callers are not in Exarust).

### Reverb (`reverb`)

- One global EAX reverb. The preset is the EFX "Generic" one with gain 0.326
  and low-frequency gain 0.5; the remaining fields are Generic.
- A level's environment record adds its decay offset: decay = 1.49 s + offset.
  The offset is applied while the reverb is on and remembered while it is off.
- Enabling needs an effect; disabling only clears the flag. There are no
  per-room zones in the original, only the per-level decay.

## Gaps

- **EFX.** Bevy's rodio backend has no EFX or auxiliary sends. The reverb
  block ([`reverb::EaxReverb`]) and the send each voice would get
  (`SourcePlay::reverb`) are computed and stored, but nothing is applied.
  `LevelReverb` feeds the level's decay offset.
- **OpenAL distance model.** The original's actual attenuation model is not
  in the decompilation pack. Bevy's spatial playback keeps its own distance
  gain; the mixer's 4 m/30 m curve is used for ranking and source assignment.
- **Listener.** The original anchors on the player's body + 50 cm and takes
  the at/up vectors from the camera. Exarust's `SpatialListener` sits on the
  chase camera, so the environment's anchor and facing come from that entity.
  Gameplay can set `AudioEnvironment::listener` directly.
- **Pitch.** Bevy exposes pitch as playback speed; the original sets
  `AL_PITCH`. Both change the effective end time the same way.
- **64 sources.** Simulated by the mixer: only bound voices get a Bevy entity
  (at most 64). Bevy itself has no fixed source pool.
- **Locks.** The original protects the cache and the source assignment with a
  spin lock and a critical section. The Rust cache and mixer are owned by the
  ECS and need no lock; the lock acquire/release functions have no separate
  counterpart.
- **Effect creation failure.** The original logs and stays silent when the
  effect slot or effect cannot be created. The failure is modelled with
  `Reverb::effect_ready`; there is no backend call to fail.
- **Full pool.** The original returns a shared dummy voice when the pool is
  full and lets callers write into it; Exarust reports no voice and the music
  and ambience code skips the write.
- **Ambience distance draw.** The horizontal distance uses FPC's `Random`
  stream, separate from the engine generator. Its seed is process-global and
  not observable; the Rust stream starts at 0 (a guess).
- **Music setting / ambience choice.** `MusicSetting` has no options screen
  yet, and only the outdoor arena ambience is chosen from the level name.

## Tests

`bank`, `environment`, `music`, `ambience`, `reverb`, `hits` and `playback`
each pin their rules to synthetic numbers: generator words, variant picks,
WAV layouts, durations, audibility ranks, sorting, bus products, source
assignment, fade curves, mirrored starts, ambience placement and the reverb
preset. The installed-sets test reads `EXANIMA_DIR` and skips when it is not
set.
