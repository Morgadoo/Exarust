## Why

PLAN Phase 7 (thaumaturgy, Force and Displacement). The force blast
(`TForceBlastEffect`, rated 79) met other bodies through a sphere stand-in
that only pushed while a chunk moved away from a limb, and never stirred the
simulated objects; the casts that read the scene under the cursor (the
throw, the barricade, the long door and the summoning) used the aim plane or
a physics ray instead of the navigation unit's ground and nearest-mesh picks
(`RF_Navigation`'s open gap after round 122).

## Evidence

REA (Exanima.exe), read against the assembly:
- The blast's three contact slots (static node, motile part, simulated
  object) and their order in the sector step's object contacts: level, sim
  objects, parts.
- The part slot: the caster's own body skipped; the share, reach scale and
  push by the body's class; the frame the part's matrix; the mesh walk (live
  chunks only, reach `reach·scale + margin + 10`, sphere then box pre-test,
  the node's sphere query with the push-out callback); the limb callback
  (barycentric weights of the chunk over the part's particles, the weighted
  motion as the velocity, the impulse pushed at those weights with the strain
  flag, the chunk's move and weight loss); then per part: energy, counter,
  knock, concern, pin gain, landing weight and a 256-tick wake.
- The sim-object slot: class 0x6, every chunk (no life test), reach
  `reach·scale + margin + 40` with the scale the last contact left, the
  node's sphere and box, each point within reach moved by
  `(1 − d²/r²)·(float)(push·0.025)` along the chunk's move, the object's
  rustle envelope adding `(1 − d²/r²)·0.0001` in double, the object woken to
  256 ticks.
- The navigation unit's ground and nearest-mesh picks and their callers: the
  throw (accept: node with an object), the barricade placement (accept: node
  without one), the long door and the summoning (accept: every node), each
  taking the hit as it stands on a floor node and 15 cm back along the ray
  otherwise; the barricade's cast testing the eye-to-pick segment against a
  running barricade's node bounds (sphere then box).

## What Changes

- exa_game: the blast's per-part contact in the part's frame against its
  node mesh (`blast_live::strike_part`), the cloth stir
  (`blast_live::stir_cloth`, `effects::blast::hit_sim_object` in the
  original's precision, no life test), the per-part knock; the cast picks
  (`thaum::cast_pick`) wired into the throw pick, the long door and
  summoning destination, the barricade placement and its dismissal;
  `nav_pick::ground_hit`'s accept test takes the node's index.
- exa_physics: `mesh_tree::sphere_contacts`, `Ragdoll::push_part_weighted`,
  `Physics::fixed_pick_where`, the `ColliderHandle` re-export.

## Out of scope

- The force wave keeps the older limb-sphere contact (`strike_chunks`).
- The unread body byte and the part-kind 0x10 node bit the part contact
  writes.
- The player's blast ending on its key's release (as on main).
- The static node's walks stay the swept level ball.
