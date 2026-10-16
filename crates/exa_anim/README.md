# `exa_anim`

Skeletons and motion sequences from `.rsq`, keyframe controllers, pose blending; produces targets for the physics character.

**Phase:** Phase 3 (see [`docs/PLAN.md`](../../docs/PLAN.md)). Status: started.

It loads `humanoid.rsq`, attaches body parts and skins, and plays clips as
the original's living sampler does (`Animator`, `src/clock.rs`): a clip
starts from a capture of where the body is and blends to its first key,
eased; loops wrap by their last key; a clip played once ends into its
follow-up with the original's blend table; the walk, run and backpedal
play in place, every other clip keeps its root's travel; a pose the body
holds pulls the sample by a weight. A physical body's tick sets the clock
rate and the capture (`exa_game::limp`); other bodies play once a frame
([`docs/formats/RSQ.md`](../../docs/formats/RSQ.md), Motion playback).

## Scope

- Parse `humanoid.rsq`: bone hierarchy (18 bones for humans) and motion sequences.
- Keyframe controllers (linear / Catmull-Rom), pose sampling and blending.
- Procedural layers: stance, aim of the weapon from mouse input, look-at, idle noise.
