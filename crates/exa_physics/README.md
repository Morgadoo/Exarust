# `exa_physics`

Rigid-body and articulated-body simulation: the core of Exanima's feel (physically driven characters, momentum-based combat, cloth).

**Phase:** Phase 3 (see [`docs/PLAN.md`](../../docs/PLAN.md)). Status: started — the original's
body solver (`body`), its level contacts (`contact`) and the character controller (`lib`).

Each fixed tick (83 Hz) moves the character controllers, then runs the
character ticks (`body::CharacterTicks`, filled by `exa_game`: clips sampled
and targets set, player last), then the body solver over every awake body
(`body::BodySystems`). Blows are queued for the next tick, and parts are
drawn between their last two ticks (`body::BodyPlugin`). Every part is
pinned at its origin to its parent, and the skeleton's links bound each
joint's range (`body::Ragdoll::new`).

## Scope

- Fixed-timestep deterministic simulation set, independent of frame rate.
- Rigid bodies (props, doors, items), collision against level geometry and convex hulls.
- Articulated character bodies driven by joint motors toward animation targets (active ragdoll / 'motile' dynamics).
- Cloth and padded-armour layers (original engine has cloth classes per material: basic, fine, padded, silk, velvet).
- Decision recorded in docs/plan: rapier3d for level colliders and the character controller; the
  body solver and its level contacts are in-house, as the original's (`docs/formats/RSQ.md`).
