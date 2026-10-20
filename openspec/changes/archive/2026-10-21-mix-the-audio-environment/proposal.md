# Proposal

## Why

`TAudioEnvironment` (13.3 KB, 44 functions) is the original's whole sound
mixer: 512 virtual voices ranked by audibility onto 64 OpenAL sources, the
listener, the bus gains, the name-keyed buffer and sound-set cache, the music
crossfader, the area ambience and the global EAX reverb. Exarust played every
sound straight through Bevy, with no ranking, no cache rules, no crossfades and
Bevy's own distance falloff.

## What Changes

- The buffer cache keys sounds by 16 lower-cased bytes, counts references,
  reuses the lowest free slot and keeps a sorted name index; sets load
  `base01`…`base16` outside it, and the variant picker never repeats the last
  one or two picks.
- The mixer keeps 512 voices, scans, fades and stops them, ranks them by
  audibility (gain below 30 m, `gain · 4 m / d` beyond), sorts them with the
  original's quicksort and binds the top 64 to source slots with the
  original's eviction, pause, refresh and restart rules.
- Every source is heard through the distance model the original sets up
  (linear clamped, 4 m to 30 m, rolloff 1). The listener sits 3 m along its up
  vector from an anchor 50 cm above the player.
- Music: play now, crossfade, overlay in and out with mirrored interruptions,
  the Music setting, and a level change that stops the main track and plays
  the level's at once.
- Ambience: the `Env<area>01` bed and the `Amb<area>` one-shots, mono ones
  placed 10–20 m away, stereo ones on the listener.
- Reverb: the stored EAX block, enable/disable and the level's decay offset,
  computed and stored (the backend has no EFX).
- Hits and footsteps submit through the mixer.

## Capabilities

### New Capabilities

### Modified Capabilities

- `audio`: the voice mixer, distance model, listener, cache, music, ambience
  and reverb rules.

## Impact

- `exa_audio` (`bank`, `environment`, `music`, `ambience`, `reverb`,
  `playback`, `hits`, `footsteps`), `exa_world::LevelInfo::reverb_decay`, the
  viewer's level resources, and `exa_game::player::Player` (it now carries the
  listener anchor).
- Evidence: REA/Ghidra decompilation and assembly of the 44 functions of the
  `TAudioEnvironment` component (the audio_env pack), the stored reverb block
  read with `read_bytes`, the OpenAL loader (source count, per-source distance
  parameters, distance model) and the transition screen (stop and forget
  before a level's track). No corpus files change; the installed-sets check
  reads `EXANIMA_DIR`.
- Out of scope: applying EFX (rodio has no auxiliary sends), the in-game
  listener orientation writer (unknown), the weather, time-of-day and story
  callers that choose the ambience, the menu theme and its alternative tracks,
  the options screen behind the Music setting, the transition screen's own
  fades, per-material sound selection, and the other crates' one-off sounds
  that still play directly.

## Phase

Phase 5, visual and audio fidelity: `exa_audio`'s set probing (01 to 16, no
repeat) and the original's audio environment.
