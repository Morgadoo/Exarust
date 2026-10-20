# `exa_audio`

Audio: PCM sound playback, positional 3D sound, reverb environments, footstep/impact/voice sets, music.

**Phase:** Phase 5 (see [`docs/PLAN.md`](../../docs/PLAN.md)). Status: skeleton.

One module per kind of sound, so each change adds its sounds as a module:
`bank` (sounds decoded by name, and the one picker every event uses to
choose a variant of a numbered sound set, `SoundSet`: its prefix and 01 to
its count, as the original names them), `music` (the level's track; a story level's track, volume, ambience and
preset by the bytes of its name, and whether entering it changes the music),
`hits` (a sword impact and a grunt at a hit), `footsteps` (steps by speed).

## Scope

- `Sound.rpk` holds plain RIFF/WAVE PCM (44.1/48/96/192 kHz, mono/stereo).
- Original engine uses OpenAL Soft with EFX reverb; target is Bevy audio or kira with equivalent environments.
- Sound sets keyed by material and event (impacts, footsteps, voices); the
  original probes a set's variants 01 to 16 and avoids repeats (still to do).
