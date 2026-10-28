# Proposal

## Why

docs/PLAN.md, Phase 3 (bodies: skeleton, physics, control), the simulation
sector (TSimSector): the static-collider grid and the simulated-object list
existed only as tested functions, the level's colliders all answered the
plain level mask, and a cloth took no contacts from bodies or the level.

## What Changes

- `exa_physics::StaticColliders`: every fixed level collider the static
  builder makes is recorded in the sector's 600 cm static grid with its
  world box (cm), piece, node and level mask (1, or 1|4 for a thin
  horizontal plate); it is emptied on a level change.
- Body detection reads those masks (`MaskedLevel`); only a living body's
  feet carry the foot mask 4 (`Ragdoll::level_mask`), so living feet skip
  thin plates and a limp body's feet meet them.
- `exa_world::cloth`: the level's cloths form the sector's object list
  (`ClothObjects`), migrate after each tick, and take body spheres and level
  triangles each fixed tick, after the body post-step.
- `sector::object_body_meet`, `StaticGrid::near_object`,
  `sector::object_pairs`, `ObjectContacts`, `body::pairs::HitEvent`.

## Evidence

The sector's grid step (assembly): the object stage tests the body's bound
sphere against the object's by the sign of `(r + r')² − d²`, then the
point-to-oriented-box distance of the object's centre to the body's mesh
box against the object's radius squared (`COMISS r², d; JP pass; JBE skip`,
so a NaN passes); the static stage runs the same two tests on the static
records of the nine cells around the object's cell; the object/object stage
offers every other object of the list to each one taking it, with no bound
test. The registration walk sets the static mask (`1.0 > half height`,
`up > 0.99` in double). The body-contacts research note records the foot
mask being cleared on collapse and death.

## Out of scope

The cloth's own body, level and object contact calls (not in this pack:
the sweep gate and own-character skip are labelled guesses), dense
per-sector tables and their overflow reset, mechanisms inside the body
step, a consumer of the hit event payload, the oriented box of a static
collider (its world box stands in), the node flag that sets the flagged
mask bit.
