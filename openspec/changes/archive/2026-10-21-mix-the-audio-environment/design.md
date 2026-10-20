# Design

## Context

The original mixes through OpenAL with EFX. Bevy 0.20's rodio backend gives a
sink per entity, spatial panning with its own inverse-square falloff, and no
auxiliary sends. Exarust keeps the original's mixer as plain state
(`AudioEnvironment`, `SoundBank`) that emits source commands, and a playback
layer that maps the commands onto one Bevy entity per source slot.

No `docs/formats/` page changes: the WAV walk is the loader's reading of
`Sound.rpk` entries, documented in the crate README.

## Decisions

- **Units.** The original is in centimetres. Positions stay in metres and are
  converted at every distance test, so the original's constants (9 000 000 cm²,
  400 cm, 3000 cm, 300 cm) are kept verbatim.
- **Distance model in the mixer's terms.** The heard gain is computed from the
  OpenAL linear clamped model and set as the sink volume; rodio's falloff is
  scaled out (positions × 0.01) so it only pans.
- **OpenAL's buffer rule.** A restart of a source still playing or paused
  keeps its buffer (OpenAL refuses a new one): paused sources resume, playing
  ones start their old buffer again. The overlay fade-out depends on it.
- **Loading by name.** Music routines take a loader so nothing loads while
  music is off and the old buffer is released before the new one is loaded.
- **Leaks and quirks kept.** Stopping A forgets its buffer without releasing
  it; an overlay's buffer is kept; an unbound outgoing A is not freed; the
  promoted volume is not written into A's nominal volume.
- **Not reproduced.** The shared dummy voice of a full pool (Exarust returns
  no voice), the locks (ECS-owned state), the runtime library's own random
  stream (a stand-in stream serves the ambience distance).

## Open reverse-engineering questions

- Who writes the listener orientation in game? Only the scene editor's writer
  is known; answer with REA cross-references to the environment's orientation
  fields from the camera code.
- The height of the player's body node the anchor is measured from; answer by
  decompiling the two anchor writers in the player update.
- The runtime library random stream's seed; answer by looking for a
  `Randomize` call at start-up with REA.
- Which `OpenAL64.dll` ships (its `AL_MAX_GAIN` default); answer by inspecting
  the shipped library's version resource.

## Risks

- The heard-gain ceiling of 1 follows the OpenAL 1.1 default; if the shipped
  library differs, gains above 1 (none are used yet) would be quieter.
