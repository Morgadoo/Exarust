# Design

## Context

The original keeps a held item as a rigid body whose parent is the hand.
The grip records sit on the item's link list, and the item's own link
iteration runs them. Exarust's item body is a separate `Ragdoll`, so the
grip links run as a solver stage (`SolverStage`) in the same iteration,
after every body's own links.

## Decisions

- The stage calls the body's own limp law (`body::limp_law`) and keeps no
  copy of it, so the float order matches the links inside a body.
- The item never meets its holder in the pair stage (`Ragdoll::held_by`).
  This plays the part of the original's all-ones group mask across the two
  bodies.
- `sync_held_striking` sets only the strike-window bit from the owner's
  swing window. The chain bit stays, so the pair stage still measures the
  item's impacts.
- The weapon-body branch skips contacts with limbs. The original runs it
  there too, but stays silent. Exarust's blade sweep already resolves a
  weapon striking a body (damage, recoil, the swing's end), and running the
  branch as well would answer the hit twice.
- The soft weld's height reads the item's mesh box (0.3 of its height),
  which is a labelled guess. Both welds are built disabled, as in the
  original, and nothing enables them yet.

## Risks

- Armed characters whose weapon model has a rigid root now carry one more
  awake body. The stage finds bodies by a linear scan for each group in each
  iteration, which is cheap for the few held items in a scene.
- A held item now collides with the level and other bodies, so a weapon can
  snag and pull the hand, as in the original.
