## Why

PLAN Phase 3/4 (the hand). A press on an item dropped from the pack (or any
prop body put into the scene after the level was built) picked the floor
under it, so a dropped item could not be lifted again: the hand's pick only
saw the level colliders, the props' stand-in colliders (built with the
level's static geometry) and the characters' limbs.

## Evidence

REA (Exanima.exe): the cursor pick walks every sector's mesh nodes that are
visible and in the view's mask (not the excluded kind), pre-tests the
segment against each node's bounding sphere (padded 5 cm for an interactive
node, else 0.1), asks the node mesh's ray query over the rest of the
segment, and keeps the nearest face; with no hit the point is on the y = 0
plane. A dropped item's node is a sector mesh node like any other, so the
original picks it. The item's put-back clears its world-contact count on
the body and the bodies below it.

## What Changes

- exa_game: the hand's pick also asks the part meshes of prop bodies with no
  stand-in collider (not held in a hand); `--gui-drop` takes several drops.
  The lift's release drops the item's world contacts and logs its countdown
  and settling ticks.
- exa_physics: `mesh_tree::ray_mesh`, `Ragdoll::clear_world_contacts`;
  debug lines for the placement query's candidates.

## Impact

Dropped items, weapons let go of and other run-time props can be lifted.
Scripted story levels and the arena duel are unchanged tick for tick.
