# Proposal

## Why

The blastfx2 lane (round 40, Phase 7 game modes and content: thaumaturgy,
force blast and torrent) reviewed the force blast and torrent against the
original. Its review found three of its own rules wrong against the
instructions and a long-standing error in the torrent's stream ribbon:

- the new `least`/`greatest` helpers kept the first operand on a NaN,
  where the original's min/max helpers are bare MINSS/MAXSS (second
  operand), and duplicated `exa_physics::engine_math::{min, max}`;
- the torrent's texture walk had been switched to single precision, where
  the emitter multiplies and adds in double;
- the stream ribbon dropped the draw's repeated last sample (so the last
  real midpoint took the whole one-sided tangent and the last two quads
  were missing), joined every stream onto the one before it with a
  bridging quad, and blended the end normals toward the doubled vertices'
  differences instead of the samples'.

Evidence: the torrent draw, its stream emitter, the blast's limb contact
and character knock, the blast light, the force geometry draw and the
engine's min/max and matrix helpers, read in the objdump listing of
`Exanima.exe`.

## What Changes

- `exa_game::effects::torrent`: the ribbon repeats the last sample, builds
  two vertices a sample with the half Hermite, emits one call a vertex but
  the last, keeps streams separate, and blends the head and tail normals
  toward the samples' end differences; a point's life share is a double
  division; the texture walk is `walk_texture` in double.
- `exa_game::effects::{blast, emissive}`, `thaum::engine`: the force
  clamps use the shared MINSS/MAXSS helpers; the lane's duplicates are
  gone. The lane's verified rules (pin gain in single then double, limb
  impulse scale in double, the trail's last width normalised, the stream
  moving mean, the force row and pass matrix) stay.

## Out of scope

- The struck character's pin gain and 5 s recoil are still not written to
  the body.
- The body-frame contact callback stays the callers' sphere and sweep.
- Nothing in the shipped content starts a blast or torrent without a cast.
