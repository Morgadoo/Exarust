# `exa_audio`

Audio: PCM sound playback, positional 3D sound, reverb environments, footstep/impact/voice sets, music.

**Phase:** Phase 5 (see [`docs/PLAN.md`](../../docs/PLAN.md)). Status: skeleton.

## Scope

- `Sound.rpk` holds plain RIFF/WAVE PCM (44.1/48/96/192 kHz, mono/stereo).
- Original engine uses OpenAL Soft with EFX reverb; target is Bevy audio or kira with equivalent environments.
- Sound sets keyed by material and event (impacts, footsteps, voices).
