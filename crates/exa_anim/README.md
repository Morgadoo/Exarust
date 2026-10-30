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
holds pulls the sample by a weight, its quaternion components interpolated
without the short-way sign check (two opposite turns half way give a NaN).
The smooth ease is (3 − 2f)·f²; the fast one 1 + 0.11f − 1/(1 + 8f), the
first sum in double and the reciprocal single, as the original computes
them. A start that replaces Walk or Run leaves a trailing-foot request
(`Animator::take_left_locomotion`), which the body's tick hands to its
footwork. A limp (muscles-off) body's sampler writes the progress as the raw clock
over the last key's time and takes the bracket at the raw clock, in the
original's branch order (the lead-in divides by the first key alone unless
the blend is positive, no guard between the keys, a NaN clock between the
keys), with no hold, the root keeping its travel (`ragdoll_bracket`,
`Animator::sample_ragdoll`), leading in from the start's capture without
ease while it aims at the capture's key and keeping the clock when the
lead ends (`Animator::ragdoll_lead`); it slerps every turn by √run toward
the gait overlay clip's sample (from the capture's turn while it leads in)
and normalises them (`ragdoll_overlay`). Before sampling it takes the
switch to a queued clip (`Animator::queued`) when the target key changes
(`Animator::last_target`): no sample, the current targets captured for the
new key (`Animator::capture_targets`, the original's capture of the
targets, shared with the living sampler's switch) and the queued clip
playing on from the same clock (`RagdollSample::Switched`). Exarust has no
clip on a corpse and queues no clip yet. The placement's (pose-only) sampler's key
pick is `placement_bracket` (no ease, no clamp, no guard between the keys,
the rebase off the capture's key) with its root source
`placement_root_from_key`, and the living sampler's switch to a queued
clip on a new target key is `switches_to_queued`; both are tested and not
called (the spawn samples with `sample`, and clips start at once). Every capture notes whether each foot
touched (`Animator::note_planted`, read back by `captured_planted`): the
muscles-off sampler's source flag while it leads in. A sample is taken in the original's
steps (the bracket, which writes the progress; the keys blended; the
gait overlay, the Run clip slerped over the Walk at the same keys; the
hold; the pose kept as the targets), so a physical body's sampler can put
its own steps between them; each tick keeps the frames it added and
whether its end test fired. A physical body's tick sets the clock rate
and the capture (`exa_game::limp`); other bodies tick by the same body
clock (83 a second, counted from the frames' time, what is under a tick
owed to the next frame), 0.6 × the race's base a tick, added in double
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
  inverse binds a mesh registers (bone `n` = joint `n`, sized by the
  highest bone number, a gap taking the mesh's own part with an identity
  inverse, each inverse relative to the mesh's part in rest), the joints
  captured in the original's walk (`capture_joints`: depth first in child
  order, the first node of a bone number setting its frame and a repeated
  bone ending its branch), and a mesh's vertex translation. The runtime
  skin controller's per-frame passes are here too, as tested rules: a
  frame's per-bone matrices (`frame_matrices`, `joint_in_node_frame`), the
  matrix blend (`blend_skin_matrix`), the second skin order that blends
  transformed positions (`skin_vertex_by_positions`, `bake_skin_positions`),
  the per-part deform that writes one entry per index with the normal
  rotated and normalised, the tangent rotated with its w kept and the
  previous-position delta (`deform_part`), the element upload size
  (`element_bytes`), the mesh's grown bounds (`bounds_from_parts`), the
  near-light vertex tint (`tint_alpha`) and the 600-tick fade-in
  (`fade_alpha`). Bevy's GPU skinning performs the per-frame deform in the
  running game; the bind uses these numbers.
  Meshes bind under an `Animator` body root or a `SkinnedRoot` one (a
  level's placed dead have no clips, but their skin still binds).
- Node keyframe tracks (`tracks`): `NodeTracks` on a scene node,
  `StartTracks` starts the tracks of a node and the nodes under it as
  using an object does (once and reverse swap at each start), and every
  frame each playing track sets its node's translation from the original's
  Catmull-Rom curve at its clock's time; F7 starts every track of the
  level ([`docs/formats/RFC.md`](../../docs/formats/RFC.md), Keyframe
  tracks). The clock's modes (free, once, reverse, loop) are `tracks::clock`'s; both
  curves (the stored Catmull-Rom and the linear controller no chunk builds)
  are `exa_formats::key_track`'s; `slerp` is the original's quaternion blend
  (shortest way, the 1.19e-7 linear fallback, sin weights from the engine's
  quick arccosine and fast sine) and is used by the
  key blend, the gait overlay and the pose sampler. `quick_yaw` is the
  engine's yaw of a flat direction over the same arccosine (`exa_ai`'s
  combat yaw and `exa_world`'s door cell step use it).
- Pose sampling and blending.
- Procedural layers: stance, aim of the weapon from mouse input, look-at, idle noise.
