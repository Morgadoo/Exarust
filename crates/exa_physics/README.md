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
joint's range (`body::Ragdoll::new`); a limp body's links also damp each
joint inside its range through their smoothed lengths, reset when the
body goes limp (`body::Ragdoll::go_limp`). In a living body every slack
link is a link muscle that turns its joint toward the clip's angle, with
the gains the character tick sets from what the character does
(`body::Bearing`) and its race (`body::RaceMuscles`); a pose drive holds
the pelvis, spine, head and feet (`src/body_muscle.rs`, `body::Drive`).
Each character tick measures a living body's balance at its chest against
its ankles and asks the level for the ground under each foot
(`contact::Level::height_below`, the original's Height Query), which give
its support S (`src/body_balance.rs`, `body::Ragdoll::measure_balance`).
Velocity retention is the original's default environment: banded by speed
for limp parts, the fast value for living limbs. A limp body sleeps by the
original's countdown (`body::Ragdoll::countdown`: 256, two off each tick
under 20 cm/s, held at 30–32 between 20 and 40 cm/s, back to 256 above)
and is woken by a blow or by going limp; a living body never sleeps.

## Scope

- Fixed-timestep deterministic simulation set, independent of frame rate.
- Rigid bodies (props, doors, items), collision against level geometry and convex hulls.
- Articulated character bodies driven by joint motors toward animation targets (active ragdoll / 'motile' dynamics).
- Cloth and padded-armour layers (original engine has cloth classes per material: basic, fine, padded, silk, velvet).
- Decision recorded in docs/plan: rapier3d for level colliders and the character controller; the
  body solver and its level contacts are in-house, as the original's (`docs/formats/RSQ.md`).
