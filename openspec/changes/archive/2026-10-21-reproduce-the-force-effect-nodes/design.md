# Design

## Shape

Each module is plain data and functions, tested without a Bevy app; the
Bevy side is not touched. Inputs the original reads from other objects
(the caster's place and ground, the power's focus rate and hit value, the
viewer's axes and relation, the GI volume's gain, the camera's backward
axis) are parameters.

## Reuse

- The ripple step and dome extent are `thaum::step_ripple` and
  `thaum::dome_extent`; the step now takes its 0.05 in double precision.
- The wave's rate, charge step, charging growth, thrown step and chunk
  separation are `thaum::{wave_rate, step_charge, charge_wave_radius,
  release_wave, separate_chunks}`.
- The Barrier's snapshots are `thaum::{BarrierPose, advance_barrier_pose,
  update_barrier_position}`; the latter lerps from the previous snapshot
  toward the current one (its first argument renamed).
- The level of detail is `effects::level_of_detail`, the yaw the engine's
  planar yaw (`nav_camera::planar_yaw`), the generators `Mt19937` (the
  RTL draw) and `thaum::engine::Roll` (the engine's 32-bit word).

## Open questions

- What the Barrier's hit value and focus rate are: set by the barrier
  power outside this group (REA, the power's creator).
- What the dome contact's body flag 1 means (named as a guess, limp): the
  body class's flag table (REA).
- Where the rain's environment colour and bright setting come from: the
  scene environment's rain call (REA, outside this group).
- The barricade avoidance's viewer figures (reach, caution, keep
  distance): the AI's record (REA, the AI lane).

No `docs/formats/` page is concerned.
