## Why

Lane `flicker` (stability). The golden suite reported CHANGED on fire-lit
shots (e01_walk_door f00780, shell f00250, menu f00780) for builds whose
changed code never ran in those scenes; main's references passed. The
lighting2 bisect showed only the torch light moving, and following any build
change.

## Evidence

- Two builds that differ only by one unrelated resource and component
  (`context/artifacts/2026-11-18-flicker/orderAB`): the `--det` traces are
  identical once body ids are set aside, and every entity id after the new
  resource is shifted (in this Bevy each resource is an entity; the resource
  hold spawns them in component id order). The pictures differ (e01 f00780:
  12.8 % of pixels by more than 3/255).
- The suspected cause, the light flicker's shared generator drawn in query
  order, is not it: with `EXA_FLICKER_TRACE=1` both builds walk no animating
  light at all (the generator never moves in these scenes).
- The fire flames seeded their streams and a fire's ring places from the
  node's entity index (and the portal's systems from the marker's or the
  body's), so the id shift reseeded every flame, and the fire light, which
  follows the flame's density, flickered with another phase.
- The original's light walk (the per-sector light activation) steps each
  sector's light list in its registration order; its particle generator is
  process-wide and clock-seeded, so no phase of the original can be matched,
  only kept stable.

## What changes

- `exa_world::flame::FlameSeeds` (one per world, own on each side of the
  bridge) numbers flames as their spawns apply; `spawn_fire` and the portal's
  systems seed from it (`queue_seeded`, `queue_reseed`, `fire_key`).
- The systems that draw one shared generator per entity, the scene's light
  and mesh registration and the light id assignment walk in entity order.
- `EXA_FLICKER_TRACE=1` logs the flicker's walk order every 30 frames.
