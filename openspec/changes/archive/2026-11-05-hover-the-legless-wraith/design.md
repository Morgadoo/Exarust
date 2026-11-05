# Design

## What the original does

- **Balance routine.** It first tests the leg count. Under two legs it
  copies the pelvis position into the feet centre and returns; the rest
  (support point, lean, score, counter, step trigger, imbalance, stepping
  end) is skipped, so a legless body's balance stays at the reset's 1.
- **Feet routine.** The flat-floor bit is tested first, then the race's
  hover byte. Hovering: one 20 cm sphere is swept 2000 cm down from 20 cm
  over the pelvis plus the misses' rise (5 cm each); a hit is the
  character's ground and clears the rise. Then has-support is set, the
  locomotion lag factor is 1, the target pelvis height is copied from the
  standing height, the lift servo runs, and support, the extension and
  the foot-spread factor are 1.
- **Standing height.** The living sampler's hovering branch sets the
  standing height to the clip root's height less 10 cm (times the
  pelvis-height factor, 1). No ground is added anywhere: the servo
  compares that height with the pelvis's own height in the body's frame.
- **Support pins.** Their targets are clamped around the feet centre the
  balance routine left, the pelvis for a legless body.
- **Combat toggle.** A player body with the hover byte has its request
  forced off.
- **Races.** Only the Wraith carries the hover flag; its model is the only
  shipped legless one with a race that moves.

## Exarust

- `Ragdoll::measures_balance` is true for a hovering race with any leg
  count; `measure_balance` wraps the routine proper in a block a legless
  body leaves after setting the feet centre; the walking feet branch
  returns for a body with fewer than two legs.
- `lift` takes `pose_heights::hover_pelvis_target` (root less 10 cm) in
  hovering mode, the root's height being the posed pelvis over the body
  root; the extension is 1 for a legless body.
- `pull_support_pins` clamps a legless hovering body around the balance's
  feet centre.
- `scene_bodies::physical_frame` includes the wraith; the viewer's level
  spawn passes the race's hover flag (the arena spawn already did).
- `PlayerBody::hovering` keeps combat mode off; the spirit's behaviour is
  built with it.

## Open questions

- Sector-local heights: Exarust's levels share one world frame; a level
  whose sector frame is not at the world's height would move the hover
  height with it in the original. Answer with a capture of a story wraith
  on a raised floor.
