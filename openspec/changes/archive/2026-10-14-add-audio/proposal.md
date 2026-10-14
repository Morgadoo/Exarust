# Proposal

## Why

Sound is part of Exanima's atmosphere and feedback (Phase 5 of
`docs/PLAN.md`); `Sound.rpk` is plain PCM WAVE, so it can play now.

## What Changes

- Bevy audio with WAV decoding; a sound bank loading `Sound.rpk` entries.
- Looping music/ambience per level, positional weapon impacts and pain
  grunts on hits, footsteps for moving characters.

Out of scope: EFX-style reverb zones, material-dependent impact and step
sets, voice sets per character, music state changes.

## Capabilities

### New Capabilities
- `audio`: playing the original sounds for music, ambience and gameplay events.

### Modified Capabilities

## Impact

`crates/exa_audio`, workspace Bevy features (`bevy_audio`, `wav`),
`crates/exarust` (listener, level music).
