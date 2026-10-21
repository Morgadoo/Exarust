# `exa_anim`

Skeletons and motion sequences from `.rsq`, keyframe controllers, pose blending; produces targets for the physics character.

**Phase:** Phase 3 (see [`docs/PLAN.md`](../../docs/PLAN.md)). Status: started.

It loads `humanoid.rsq`, attaches body parts and skins, and plays clips as
the original's living sampler does (`Animator`, `src/clock.rs`): a clip
starts from a capture of where the body is and blends to its first key,
eased (a stagger's get-up jumps in a few keys, `Animator::jump_in`, and
holds its first key); loops wrap by their last key; a clip played once ends into its
follow-up with the original's blend table; the walk, run and backpedal
play in place, every other clip keeps its root's travel; a pose the body
holds pulls the sample by a weight. A sample is taken in the original's
steps (the bracket, which writes the progress; the keys blended; the
gait overlay, the Run clip slerped over the Walk at the same keys; the
hold; the pose kept as the targets), so a physical body's sampler can put
its own steps between them; each tick keeps the frames it added and
whether its end test fired. A physical body's tick sets the clock rate
and the capture (`exa_game::limp`); other bodies tick by the same body
clock (83 a second, counted from the frames' time, what is under a tick
owed to the next frame), 0.6 × the race's base a tick
([`docs/formats/RSQ.md`](../../docs/formats/RSQ.md), Motion playback).

## Scope

- Parse `humanoid.rsq`: bone hierarchy (18 bones for humans) and motion sequences.
  Without the file the library is the original's fallback: one unnamed
  action and one empty `Default` moveset, so characters play nothing
  (`Motions::fallback`); Exarust takes the same fallback when the file
  does not parse.
- Clip tables: an `Animator` names its base moveset (the race's own,
  `Default` for a human) and the held items' `MovesetConfig`;
  `Motions::clip` looks every clip up in the table the original assembles
  from them (`exa_formats::rsq::MotionSet::assemble`), built once per base
  and config and shared ([`docs/formats/RSQ.md`](../../docs/formats/RSQ.md),
  A character's clip table).
- CPU skin data (`skin`): the skin matrix (joint world × inverse bind),
  one vertex deformed in the original's order (the joints' matrices
  blended by weight up to the first joint number below 1, then one
  transform summed `(x·a + y·b) + (z·c + t)`), the joint numbering and
  inverse binds a mesh registers (bone `n` = joint `n`, a gap taking the
  mesh's own part with an identity inverse, each inverse relative to the
  mesh's part in rest), and a mesh's vertex translation. Bevy's GPU
  skinning performs the per-frame deform; the bind uses these numbers.
- Node keyframe tracks (`tracks`): `NodeTracks` on a scene node,
  `StartTracks` starts the tracks of a node and the nodes under it as
  using an object does (once and reverse swap at each start), and every
  frame each playing track sets its node's translation from the original's
  Catmull-Rom curve at its clock's time; F7 starts every track of the
  level ([`docs/formats/RFC.md`](../../docs/formats/RFC.md), Keyframe
  tracks). The clock's modes (free, once, reverse, loop) and both curves
  (the stored Catmull-Rom and the linear controller no chunk builds) are
  `exa_formats::key_track`'s; `slerp` is the original's quaternion blend
  (shortest way, the 1.19e-7 linear fallback, sin weights) and is used by
  the key blend, the gait overlay and the pose sampler.
- Pose sampling and blending.
- Procedural layers: stance, aim of the weapon from mouse input, look-at, idle noise.
