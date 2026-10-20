# Design

## Behaviour

The door and lever frames are built the same way: the axis from the pins, then the anchor of the swing link (its tied point less the node's position) with its part along the axis removed and normalised as the third row, and the axis crossed with it as the first. The heading of a point is measured from the third row toward the first. The anchor's own heading is therefore 0, which settles the earlier inference: a door's closed angle is exactly its half swing and a lever's detents are at −α and +α.

Thresholds and sound factors that the original keeps as doubles are compared or multiplied after widening the single-precision value. They are spelled as doubles in the code, since `0.05_f32 as f64` is not 0.05. This is visible at the edges: the float nearest 0.95 is below the double 0.95, so a single leaf there leaves, and a plate grind speed of the float nearest 0.001 still grinds.

The load fire's record carries its size and lift in the two halves of its first target word; a signalled fire job carries its size in the record's value. The switch-off flag is applied first and does not end the setup.

## Model

- `door::shape`, `door::half_swing`, `door::Door::{step, stored}`; `lever::shape`, `lever::half_swing`, `lever::lever_step`; `plate::plate_step` with `Plate::state`.
- `mechanism::{level_setup -> LoadSetup, daylight_target, job_fire_size, fire_lift, latch_doors, close_and_lock_doors, toggle_doors}`.
- `exa_world::mechanisms::MechanismRecords`, inserted on placed objects with a door or operative section.

## Open questions

- Which of the two clocks the door's knock and slam cooldowns read and which the unlock timer reads (REA: the clock globals' writers).
- Whether the init frame and the step's angle (body point less centre, through the node matrix) agree for a rotated node (REA plus a capture of a rotated door).
- The node flag that gates a lever's timed return (REA: its writers).
- The world-link law, for M33.

## Docs

`docs/formats/` is unchanged: no record field changed. `crates/exa_physics/README.md` and `crates/exa_world/README.md` describe the mechanisms and the records.
