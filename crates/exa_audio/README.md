# `exa_audio`

Audio: PCM sound playback, positional 3D sound, reverb environments, footstep/impact/voice sets, music.

**Phase:** Phase 5 (see [`docs/PLAN.md`](../../docs/PLAN.md)). Status: the
sound bank and its numbered sets, the original's virtual-voice mixer, its
OpenAL distance model, music crossfades, area ambience and the reverb
parameters are in. EFX is not applied by the backend (see [Gaps](#gaps)).

## Modules

| module | what it holds |
|---|---|
| `bank` | the name-keyed buffer cache (`Sound.rpk` WAV files), numbered sets, the variant picker and the engine generator |
| `environment` | the 512-voice pool, listener and bus gains, audibility ranking, the quicksort, source binding, the per-update state machine and the OpenAL distance model |
| `music` | the two-track crossfader (A/B), its four states and the level's track; a story level's track, volume, ambience and preset by the bytes of its name, and whether entering it changes the music |
| `ambience` | an area's looping bed and its scheduled one-shots |
| `reverb` | the global EAX reverb parameter block and the level's decay offset |
| `hits`, `footsteps` | the game's first two users of the mixer; the footsteps read the body's gait plants (`exa_physics::body::Footstep`) with the original's gain and pitch, and keep a speed timer for bodies without a ragdoll |
| `sounds` | the body's named sets: `SwingHA`, `StmHeal01`, the hit classes (`BdyCol`, `SkelHit`, `HitGlm`, cloth, leather, mail, plate) and the foley loops (`FoleyPlH`, `Golem_Foley`) |
| `loops` | the looping sources a character keeps: one per held item and its body's voice and foley pair |
| `bodies` | the plain rigid bodies' collision one-shots and scrape loops (`exa_physics::prop::BodySounds`) |
| `playback` | one Bevy audio entity per real source slot, driven by the mixer's commands; the listener anchor |

## Rules recovered from the original

### Buffer cache and sound sets (`bank`)

- A sound's identity is the first 16 bytes of its name with `A`–`Z`
  lower-cased, zero padded. Longer names collide with their 16-byte prefix.
- The buffer table reserves slot 0 as "no buffer". A load takes the lowest
  free slot from 1, growing the table by one when none is free, with one
  reference. A load by name inserts the name into a key-sorted index; a hit
  adds a reference; a miss is not remembered.
- A release subtracts a reference; at zero the name leaves the index (whatever
  slot its entry names), the slot and its asset are dropped and the slot is
  reused.
- The WAV walk: the `WAVE` tag at byte 8 (`RIFF` is not checked), chunks
  while a header starts more than 8 bytes before the end, no word padding. A
  `fmt ` chunk must be PCM and is stepped past by twice its size less 16
  (exact for the usual 16-byte chunk); `data` counts only after it. Without
  `fmt ` the defaults are mono 16-bit 22 050 Hz.
- Durations come from the header: mono 8-bit counts a data byte as
  `1/frequency` seconds, mono 16-bit and stereo 8-bit as `0.5/frequency`, and
  stereo 16-bit as `0.25/frequency`, rounded to whole milliseconds (half to
  even). The seconds form used for music offsets rounds `bytes · 8 /
  (channels · bits)` to whole frames, then divides by the frequency.
- A set is built from `base01`, `base02`, … stopping at the first missing
  file or at 16 variants, each loaded into a fresh slot outside the name
  index. A base name must be 1 to 14 bytes. Sets keep a reference count and
  are shared by base name; releasing the last holder releases every variant.
- A variant pick steps the engine's generator (`state ← state·0x08088405 +
  1`, index = high half of `state · count`). One variant always picks 0
  without stepping; two or three never repeat the last pick; four or more
  never repeat either of the last two. A fresh set has both last picks at 0,
  so its first pick skips variant 01. The engine's unit draw is the stepped
  word times 2⁻³², narrowed to single precision.

### Mixer (`environment`)

- 512 voices, 64 real sources (the original makes up to 64). Allocation scans
  upward from a free hint, marks a voice allocated, gives it source −1 and
  raises the high-water mark. A free stops its source, detaches its handle,
  releases a buffer it owns, clears its flags and lowers the hint. Other
  fields are not cleared on reuse.
- One-shots (from a buffer, from a set, or attached to a handle) need the
  environment to be live; a set one-shot picks its variant before taking a
  voice, an attached one after. Retriggering an existing voice does not ask.
  Finishing a voice marks a playing non-looping one auto-free (its handle
  stays attached until it is freed) and frees anything else at once.
- An update runs, in this order: the ambience one-shot scheduler (while an
  area is set, even an empty one), the music state machine, the window-focus
  step, the bus gains, the listener, the voice scan, the audibility sort,
  source assignment, the voices beyond the top, then stopping every source
  left unused.
- Focus falls by 0.02 per update while the window is unfocused and rises by
  0.01 to 1, both steps in double precision. The interface bus is master ×
  transition × focus; the world bus is the interface bus × the world fade.
- The listener sits at the anchor plus 3 m along its **up** vector (the
  second half of the orientation it sends). In the original game the anchor
  is the player's body plus 50 cm up. The original is in centimetres;
  gameplay is in metres, so distance tests convert to centimetres and keep
  the original constants.
- The scan: a restart request activates a voice; fading voices lose one gain
  per second of real time and stop below `f32` epsilon; a stop clears the
  active, restart, stop and loop bits and frees an auto-free voice; a
  non-looping voice whose source has finished is freed (auto-free) or
  deactivated.
- Audibility is −100 when inactive, the gain for a listener-relative voice,
  the raw gain below 30 m, and from 30 m `gain · 4 m / d`. The original's
  "inside 4 m" branch sits inside the "beyond 30 m" test and can never run;
  it is kept in the same place.
- The sort is the original's Hoare quicksort, high first, with its exact swap
  order for equal ranks.
- The active count is used as the last top index (not count − 1), so one
  extra voice is looked at; it is inactive by construction. At 64 or more
  actives the last index is 63, giving exactly 64.
- While the game is paused, top world voices are paused and interface voices
  refreshed; otherwise paused voices resume and every bound top voice is
  refreshed. Top voices without a source and not stopping take the first
  unused slot.
- Binding evicts the slot's previous voice (it loses the slot, then an
  auto-free one is freed and any other deactivated). It then attaches the
  buffer, sends world voices to the reverb when enabled, sets the gain (voice
  gain × bus, clamped to 4), the pitch, the position (absolute, or
  anchor-relative), the minimum gain (voice gain × interface bus) when the
  voice defeats distance attenuation, the looping flag and the end time
  (`round(duration / pitch)` ms on), and starts it unless paused. The start
  offset is consumed once. A refresh updates the gain (its minimum gain is
  the clamped gain), pitch and position every frame and re-attaches and
  restarts the buffer when a restart was requested.
- Voices beyond the top: auto-free ones are freed; others lose their active
  bit, and a looping one keeps a restart request so it resumes when it ranks
  high again.
- The music and ambience callers take the shared dummy voice when all 512
  voices are in use (the original's other allocation): it has flags 0 and
  source −1, is never ranked or bound, and writing to it changes nothing
  audible. The one-shot and ambience one-shot paths keep the allocation that
  reports no voice; the music and ambience callers' writes to the dummy are
  dropped by the holder.
- Every source is heard through OpenAL's linear clamped distance model with
  reference distance 4 m, maximum distance 30 m and rolloff 1: full gain
  inside 4 m, falling linearly to silence at 30 m. The heard gain is held
  between the source's minimum gain and OpenAL's default maximum of 1.

### Body sounds (`sounds`, `loops`)

- A fighter's swing voice (`swing_voices`, from `exa_combat`'s
  `SwingVoiceCue`): the first attack tick in the swing's region restarts the
  voice of its first held item on a `SwingHA` variant, silent, at 0.4 × W26,
  and rustles its clothing foley (volume × (0.02r + 0.02), pitch
  0.4r + 0.8, then 400 ms plus a jitter under 100 before the next rustle);
  every attack tick sets the voice's volume W25 · 0.07f and pitch
  W26 · (0.5f + 0.4) from the tracked tip speed (f = 0.1·√speed², faded
  past 80 % of the clip; a stab has its own curve); a tick that is not an
  attack fades a playing voice by 0.01 and 0.0005 and stops it under
  0.005. Fists and claws are not held items and swing silently.
- A second wind's heal plays `StmHeal01` at 0.1 and pitch 1.
- Named one-shots (`sounds::named_sounds`, each buffer loaded once): a
  door's lock sound (`exa_physics::mechanism::LockSounded`, the `LkO` an
  order to open the doors under a node plays) draws its pitch first,
  `0.9 + 0.15·draw`, then its volume, `0.15 + 0.1·draw`, from the engine's
  generator, in double precision (`lock_sound_levels`); a `PointSounded`
  plays at its own volume and pitch (a taken object's use sound, 0.25 and 1).
- A blow plays the weapon's own set by its record's hit-sound class
  (`WH_SwordA`, `BdyCol` for fists, …) and the struck part's material set:
  its outer armour layer covering at least half, else the inner, its class
  (`foley` classes 2 cloth, 3 leather, 4 mail, 5 plate), the flesh
  (`BdyCol`) or skeleton (`SkelHit`) default, or `HitGlm`; the class rules
  are `exa_combat::sounds`. The part's sound falls silent when no layer
  covers half. A blow no longer grunts by itself.
- The balance's foot sounds (`exa_physics::body::FootSounded`, from the
  standing footwork): a step's start plays the character's foley set at its
  standing place (`anchor_rustle`: the volume's draw, the pitch's, the
  play, then its 400 ms clock's jitter), a step's end its race's step set at
  the body root's place (`release_step` at the step loudness and footstep
  volume, then its 250 ms clock), each only while its clock is behind the
  game clock; the draws come from the sound bank's generator in that order
  (`footsteps::foot_sounds`, logged at debug level).
- A wound's pain sounds (`exa_combat::pain::PainSounded`, after the damage
  call on flesh only): the character's wound set (`WndFls`, `SkelHit`,
  `HitGlm`) at the blow, and its pain voice through its voice's request
  (the short or heavy groan set of its own voice, its waits, volume ×
  voice volume × sound gain, pitch × voice pitch) at its place.
- A character keeps a looping source per held item and a pair for its body
  (voice and foley), made lazily and freed on death (`LoopSources`);
  `play_loop_attached` binds a looping variant to a caller-held handle,
  and `play_buffer_attached` a named buffer (a fire's loop,
  `exa_game::fire_sound`).
- A clothing foley loop is chosen by the summed coverage per hit-sound
  class times the per-bone weights (`FoleyPlH`) and its volume by the
  class's volume factor; a golem body plays `FoleyPlH` at 2, or
  `Golem_Foley` at 6 (1 for race 0x1200).

### Rigid bodies (`bodies`)

- The first block of a swing on a held shield or torch
  (`exa_combat::block::ShieldRung`) plays the item's root part's material
  collision set (`BodySounds::impact`, which held items now carry,
  `exa_character::held_item_sounds`) at the blow, at the ring's volume and
  pitch (`hits::shield_sounds`).
- Each body tick (fixed step, after the solver), a prop's pair impacts
  (`Touch::struck`, sphere side then touched side, in pair order) and then
  its level impacts go through the plain body's collision call
  (`exa_physics::body::impact_sound`) with the part's material set
  (`<prefix>col`, e.g. `wdlcol`, `mtlcol`, numbered 01 on). A sound is a
  one-shot at the part's mesh centre in the world (the original's body
  centre). A part whose set has no shipped variant is silent. Held items
  and characters are left to `exa_combat`.
- Each scrape source drives one attached loop: over the speed floor a loop
  that is not playing starts at volume and pitch 1, then takes the event's
  volume, pitch and point; under the floor a playing loop gets the stop
  bit. A body that leaves the world frees its loops.
- [inf] The cooldown draw comes from this system's own engine-style
  generator, not one shared with every other random draw.

### Music (`music`)

- Two tracks: A is playing, B is incoming or the overlay. All fades last
  3000 ms on the real-time clock.
- Playing a track on A now remembers its name and volume and, while music is
  on, releases A's old buffer, loads the new one by name and restarts A's
  voice on it. The level's track is played this way after the transition's
  stop: a level change stops A and forgets its name, then plays the level's
  track at once.
- A crossfade forces music on: B rises linearly from 0 while A falls as
  `(1 − t)²`; at the end A is freed (only if it had a source) and its buffer
  released, and B becomes A.
- An overlay fade-in starts B at a random point in the first 80 % of its
  length, ducks A as `(1 − t)²`, and pauses A's source at the end. A fade-out
  restarts A, which resumes the paused source, and frees B at the end.
- Starting either fade during the other mirrors its progress (`now − round((1
  − t)·3000)`, in single precision), so the gains continue without a jump.
- The Music setting acts only when A's volume is above 0.01: off stops A, on
  plays the remembered name again at the remembered volume. The compare is
  unordered-safe, so a NaN volume is left alone, as the original's is.
- The transition screen reads and writes A's voice gain (`music_gain`,
  `set_music_gain`) to fade the music out with the screen, then stops A and
  forgets its name at black. The story's level-entry check
  (`story_music_changes`) compares the destination's track with the playing
  one on 15 characters; `story_music_changes_for_key` makes the same check
  against Exarust's lowercase track key (`music_name`).
- Observed quirks kept: the promoted track's volume is not written into A's
  nominal volume (the original writes it to a global nothing reads); B's
  buffer is neither released nor forgotten when an overlay ends; stopping A
  forgets its buffer without releasing it and leaves a running crossfade
  alone.

### Ambience (`ambience`)

- An area `X` has a bed `EnvX01` at `0.15 × volume`, pitch 1, looped on the
  interface bus, started at a random point in the first 0.8 s, and a one-shot
  set `AmbX01…16`. Both names pass through a 15-*byte* short string.
- Setting an area releases the old bed buffer only when it changes and the
  old set after loading the new one; the bed voice is reused.
- The one-shot scheduler runs on the game clock. The first shot is due
  15–45 s after an area is installed. A mono shot is placed 10–20 m from the
  listener anchor in a random horizontal direction, at gain 0.02–0.05 for
  the first two variants and 0.03–0.10 for the rest, pitch 0.9–1.1, and is
  rescheduled 5–15 s later. A stereo shot stays on the listener, at gain
  0.075–0.15 and pitch 0.8–1.0, and is rescheduled 15–45 s later. Every
  range is a unit draw scaled in double precision.
- The mono direction is the two draws taken off 0.5 and normalised in single
  precision; a squared length at or below the `f32` epsilon, or an unordered
  one, sets only the first component to 1 and keeps the second draw's value.
  The direction is scaled in centimetres and only then converted to metres.
- A story level takes the area its name's bytes pick at full volume: the
  fourth level names `AmbUWBA`, whose prefixes then build `EnvAmbUWBA01` and
  `AmbAmbUWBA`, files that do not ship (the original's naming bug, kept);
  every other story level clears the ambience.
- Only the outdoor arenas' calm daylight case is wired: `NtrA` at volume 1
  (a still, dry day; the weather and time-of-day callers are not in Exarust).
  `arena_ambience` holds the recovered full rule — rain > 0.01 stops the
  ambience, daylight < 0.5 gives `NtrAN` at 0.3, a calm arena gives `NtrA` at
  1, wind > 0.1 gives `NtrW` at 1 with bed gain `wind² × 0.2` and pitch
  `0.6 + 0.5 × wind`, otherwise none — but the per-arena calm flag comes from
  the arena progression's tables, which are not ported, so nothing calls it
  yet.

### Reverb (`reverb`)

- One global EAX reverb. The stored block is the EFX "Generic" preset with
  gain 0.326, low-frequency gain 0.5, late reverb gain 1.259 and air
  absorption 0.994; every field is pinned bit for bit.
- A level's environment record adds its decay offset: decay = offset + 1.49 s
  (double precision). The offset is applied while the reverb is on and
  remembered while it is off.
- Enabling needs an effect; disabling only clears the flag. There are no
  per-room zones in the original, only the per-level decay.

### Backend (`playback`)

- One entity per source slot. Spatial sources pan through Bevy's spatial
  sink with rodio's falloff scaled out (positions × 0.01), so the volume is
  the mixer's OpenAL distance model alone.
- A restart of a source that is still playing or paused keeps its buffer, as
  OpenAL refuses a new buffer then: a paused source resumes, a playing one
  starts its old buffer again. A stopped or finished source takes the new
  buffer.
- The listener anchor is the entity with `ListenerAnchor` (the player) plus
  50 cm up, else the `SpatialListener` (the chase camera); the facing and up
  vectors come from the `SpatialListener`.

## Gaps

- **EFX.** Bevy's rodio backend has no EFX or auxiliary sends. The reverb
  block ([`reverb::EaxReverb`]) and the send each voice would get
  (`SourcePlay::reverb`) are computed and stored, but nothing is applied.
  `LevelReverb` feeds the level's decay offset.
- **Listener orientation.** The original takes its at/up vectors from the
  camera in the scene editor; no in-game writer is known. Exarust takes them
  from the chase camera, so the listener sits 3 m along the camera's up
  vector from the player's anchor, and panning is Bevy's, from the camera's
  ears.
- **Anchor height.** The original's anchor is the player's body node plus
  50 cm; Exarust's player root is at the feet, so the anchor may sit lower
  than the original's (a guess until the body node's height is known).
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
- **Full pool.** The music and ambience callers take the shared dummy voice
  when all 512 voices are in use, as the original does; it is never ranked or
  bound, so its writes are inaudible. The one-shot paths report no voice.
- **Arena ambience table.** The per-arena calm flag and the night draw's
  arena index come from the arena progression, which is not ported; the
  viewer keeps the calm daylight `NtrA` choice. `arena_ambience` holds the
  rest of the recovered rule.
- **Buffer queries.** The music overlay's length query passes the table slot
  where OpenAL expects its own buffer name; both count up from 1 and reuse the
  lowest free one, so they agree, and Exarust reads the slot.
- **Ambience distance draw.** The horizontal distance uses the runtime
  library's own random stream, separate from the engine generator. Its
  generator and seed are outside the evidence; Exarust uses a stand-in stream
  from 0.
- **Music setting / ambience choice.** `MusicSetting` has no options screen
  yet, and only the outdoor arena ambience is chosen from the level name.
- **Transition fade.** The transition screen's own fade of A and of the
  transition bus is outside this crate; a level change cuts A at once.

## Tests

`bank`, `environment`, `music`, `ambience`, `reverb`, `hits`, `bodies` and `playback`
each pin their rules to synthetic numbers: generator words, variant picks,
WAV layouts and walk steps, durations, frame-rounded lengths, audibility
ranks, the distance slope, sorting, bus products and focus steps, source
assignment, eviction, pause, handles, fades, mirrored starts, ambience
placement and names, the arena weather choice, the reverb block's stored bits
and the backend's restart rule. The installed-sets test reads `EXANIMA_DIR`
and skips when it is not set.

`voices` holds the original's voice choice (a body with a feral level above
0 speaks its undead voice, else its living one, with the record's sex bit)
and the footstep set selection (the golem suit's `Golem_Stp`, else the
race's step name), plus the name of the wet splash set (`StpWet`). The
footstep sets are now chosen from the race record (`CharacterStats::
footsteps`, its first five names): a physical body's sampler sends a
`Footstep` message for each foot it plants, and `footsteps` loads the set,
draws the engine's unit twice (the pitch's p, then the gain's g) and plays
`gain = (0.5g + 0.5)((0.08·speed)² + 0.01)`,
`pitch = (0.2p + 0.9)(0.05·speed + 0.85)` at the plant's place. The two
body gain fields the original multiplies in have no Exarust counterpart
and are 1 (a guess), and the underwater splash and the foley set are not
in. Nothing selects voices in the running game yet.
