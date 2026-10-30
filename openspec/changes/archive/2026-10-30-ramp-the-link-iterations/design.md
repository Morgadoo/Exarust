# Design

- The femur posture routine writes each leg's thigh slot (the leg part's
  parent), which the living sampler multiplies into the thigh's clip turn;
  Exarust computes the same turn per sample (`femur_posture_turn`). Nothing
  turns the feet.
- The posture block's retention share is (1 − balance)(1 − r) + r with r the race's limb ratio;
  `retention_factor` already computes it where the living retention reads
  it, so a second stored copy is not kept.
- The link-iteration count only marks the solve's passes and sizes the
  whole-count iteration, so the ramp changes no running motion; it is wired
  because it is one comparison a tick. A limp body has no drive and is not
  ticked, so death's 10 stays.
