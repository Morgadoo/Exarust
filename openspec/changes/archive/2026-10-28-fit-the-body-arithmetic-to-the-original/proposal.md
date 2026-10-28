# Proposal

## Why

A further pass over `TTetraRBD` and `RF_Dynamics_Volumes` against the original's instructions found four arithmetic and state details where the body kernel and the volume fit differed from the original, and one misreading that the review corrected before it landed.

## What Changes

- The edge restore takes an edge's length as `√((x² + z²) + y²)`, the original's own sum, for both the current and the rest length, and corrects by `(len − rest)/(len + len)` times the gap, both ends.
- The Verlet step adds the tick's kept motion to a particle and then its gravity displacement, as two rounded sums.
- A body that falls asleep empties each part's motion and smoothed-velocity buffers.
- The volume quicksort is written in the original's form (pivot halfway between the current low and the call's fixed high, the left side recursing and the right looping); the permutation is unchanged.
- The coverage discs add the squared offsets as `z² + (y² + x²)`.
- The neighbour scans read all 26 neighbours: the lane's 25-entry reading (and an option for an "older exporter") is withdrawn, since the original's loops test their index after the body.

## Capabilities

### New Capabilities

### Modified Capabilities

- `physical-body`: the edge restore, the step's adds, the sleep buffers.
- `object-physics`: the volume sort's form and the neighbour scans.

## Impact

- `exa_physics::body` (`keep_rigid`, `Ragdoll::integrate`, `Ragdoll::post_step`) and `exa_physics::volume` (`sort_by_key`, the coverage discs, the neighbour table).
- Live: the body changes run in every level (props, corpses, thrown items); they change rounding only, and a woken body's smoothed velocity starts from zero.
- Evidence: the edge-restore routine's lane sums and correction, the rest lengths' distance helper, the step's per-particle add sequence, the post-step's sleep branch clearing three buffers, the quicksort's scans, and the neighbour loops' increment-then-compare form. The shipped `Objlib` check still fits 1060 of 1503 stored lists bit for bit with the 26-entry scan.
- Out of scope: the third motion buffer, the frozen-body branch, the per-root iteration count, the group mobility blend, audio hookup and wiring the volume fit into a live builder.

## Phase

Phase 3, bodies: skeleton, physics, control.
