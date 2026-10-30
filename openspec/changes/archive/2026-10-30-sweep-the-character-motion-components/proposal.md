# Proposal

## Why

A sweep of the small "Characters, motion & animation" components
(Phase 2 — Characters and animation, see docs/PLAN.md) ported the skin
controller's CPU passes, the motion unit's part-id walk and rebuild
transfer, a texture layer's strength curve, the golem alarm return and the
fall scream through the character's own voice. The review against the
assembly found the joint placed in the node's frame turned the wrong way,
the inverse bind applied after the joint instead of before, the bounds'
middle taken as an average instead of min plus half the span, a part
transfer that skipped source parts without a controller and copied a
tree-wide word, an id walk that entered children of unnumbered parts, a
shock recovery that repeated the existing force release, and an alarm
return that repeated the radius query without its AI race-word check.

Evidence: REA (Ghidra) pseudo-code and the objdump listing of the skin
controller's per-frame deform, previous-position priming, in-place bake,
origin shift, bounds pass and near-light tint, and of the engine's matrix
product, inverse placement, interpolation, length and normalise helpers;
the motion unit's id walk, start, role copy, id copy and state copy; the
layer strength routine and the fast logarithm and exponential; the shocked
state's release; the alarm return and its radius query.

## What Changes

- `exa_anim::skin`: `compose`, `inverse_transform_vector`, `length`,
  `prime_previous_positions`, `shift_previous_positions`;
  `joint_in_node_frame` and `frame_matrices` on 12-float matrices in the
  original's order; the bounds' middle; `skin_vertex` reuses
  `blend_skin_matrix`.
- `exa_anim::parts`: `PartController`, the role copy and the id walk as the
  original runs them, and `copy_by_id`.
- `exa_assets::paint::layer_strength` (verified, unchanged).
- `exa_physics::shock::recover` removed (it repeated `force_release`).
- `exa_ai::golem::alarm_return` through `radius_query` and a homeward
  callback.
- `exa_game::fall`: the scream requests line 2 of the character's voice.

## Out of scope

- The skin controller's GL buffer prepare, attach and submit (Bevy's GPU
  skinning draws the bodies); none of the skin passes is called live.
- The texture layer list's slot assignment, refcounting and material
  updates; the layer strength has no caller yet.
- Rebuilding a body's model in play (the part transfer has no caller), the
  alarm return's save-and-leave caller, the live arc shock calling
  `exa_physics::shock::shock`, and attack voices.
