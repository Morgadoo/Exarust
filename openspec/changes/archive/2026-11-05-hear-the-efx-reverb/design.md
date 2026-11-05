# Design

Extends the audio notes in the crate README (`crates/exa_audio/README.md`,
Reverb and Gaps); no `docs/formats/` page describes audio behaviour.

## What the original does

- One auxiliary effect slot and one `AL_EFFECT_EAXREVERB`, created on the
  first enable. The block is the EFX Generic preset with gain 0.326, LF gain
  0.5, late gain 1.259 and air absorption 0.994. Decay = 1.49 s + the level
  record's offset, re-uploaded and reattached whenever the offset changes
  while the reverb is on.
- Each upload is field by field; OpenAL refuses an out-of-range value, so a
  decay outside 0.1 to 20 s leaves the effect's previous decay.
- A world voice gets send 0 to the slot when it is bound while the reverb is
  on, with no send filter; interface (music, bed) voices never do. The send
  is not touched again until the source is rebound, so disabling keeps the
  current sources sending.
- The scene builders and the indoor arena preset enable the reverb; the
  outdoor arena preset disables it. Every level record read so far
  (`exanima01`, `exanima02`, `arena1`, `arenahub`, `arenao3`) has offset 0.
- OpenAL Soft defaults the original keeps: slot gain 1 and automatic send
  gain; source room rolloff 0, air absorption 0; listener metres per unit
  1, speed of sound 343.3. The engine's units are centimetres, so OpenAL's
  initial-decay attenuation of the send (`0.001^(d / (decay × 343.3))`, d
  the clamped distance past 400 units) is counted in centimetres: the
  reverb is heard from sources within a few metres of the listener.

## Route

OpenAL Soft through a binding would be closest, but the toolbox has no
`libopenal` and nothing can be installed without sudo. The reverb is
therefore ported into Exarust's mixer: a Bevy audio source of its own (the
bus) runs the reverb on rodio's audio thread, and the dry sources stay as
they were. Rodio sinks cannot be tapped, so the bus decodes each sending
buffer once and plays a wet copy with the same commands.

## Open questions

- Is the reverb's level right against the retail game? Answer with a
  capture of the retail game's output in a room (retail-exanima skill) and
  the same position in Exarust with `EXA_EFX_RECORD`.
- The dry and wet starts are a frame apart (the bus receives its command on
  the audio thread's next poll, the dry sink when Bevy spawns it). Measure
  the offset in a recording if it is audible.
