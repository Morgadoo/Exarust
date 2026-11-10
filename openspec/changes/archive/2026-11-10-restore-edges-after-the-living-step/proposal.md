## Why

Phase 4 (Combat and items) in docs/PLAN.md, TMotilePart, the motilepart3
gap lane (round 152). The round-141 routine table left the living limb
step, the muscle setup and the link iteration's saturation partial. An
audit of those routines against the assembly found three pieces Exarust
did not do as the original: the edge pass the tree's root runs at the end
of its living step, the link muscle's damping and saturation arithmetic,
and the muscle setup's link smoothing precision.

## Evidence

REA (Ghidra and the objdump listing of `Exanima.exe` 0.9.5):

- The living limb step, on the tree's root, ends by restoring the six
  edges of every part of the tree (its own, then its children's, in the
  tree's order), then remembers the muscles' anchors and keeps the group
  awake. Bodies join their sector's list node first, children after (both
  the scene scan and the sector migration walk the node tree that way),
  and the sector step walks that list from its end, so a tree's root is
  the last of its parts to step: the pass comes after every part's
  inertia, carry and pose drive. The anchors are read on the tick's start
  particles, so their order against the pass does not matter.
- The link iteration's muscle: the damping ratio is k over (0.5|e|² +
  1.4) with the spread in single precision and the floor and division in
  double; the pelvis and legs take the double 0.6 on that ratio before D,
  all in double; the others multiply ratio and D in single precision.
  The saturation helper takes the reciprocal root of |e|² with RSQRTSS,
  adds the saturation, takes RCPSS of the sum and scales the error by
  their product.
- The muscle setup's smoothing: r² summed (x² + y²) + z² in single
  precision (the pseudo-code's order is wrong), none when it compares at
  most the double 0.1 or is unordered, else (1 − (r² − 25)·0.001)² in
  double, rounded once, and 0.99 when that compares over the double 0.99.
- The health write-back is already ported and called by the save
  (`CharacterData::save_health`); its seed write is an identity call on
  0xFFFF, as Exarust writes it.

## What Changes

- `exa_physics` body: `Ragdoll::step` restores every part's edges after
  the pose drive in a living body.
- `exa_physics` muscles: `damping_term`, `saturate` and `MuscleGains`;
  `muscle_pull` and `Link::muscle` use them.
- `exa_physics::engine_math::reciprocal_approx` (RCPSS).
- `setup_smoothing` in the original's precision and order.

## Out of scope

- The slot-37 hit event for fists, the player's target component, the
  living step's placed (0x1000) and frozen branches (no living character
  is placed or frozen in Exarust), the add-to-world pin and contact
  re-scale by the character size.
- The combat-rule functions and the character creator.
