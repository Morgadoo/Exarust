## Why

PLAN Phase 3/4 (the hand). The lift held the item with a spring grab, awake at
256, so the release's raise to 128 never showed and a neighbour woken at the
old place took 256. The lift's placement ray saw only the props' stand-ins,
so an item could not be set on a dropped item. A marker the item would
overlap was refused by a sphere-against-sphere stand-in.

## Evidence

REA (Exanima.exe):
- The lift's deferred take-out sets the body's state |= 0xc05 (asleep, the
  state-4 bit, a transient bit, disabled), zeroes its world contacts, gives
  it the inert environment, recursively over its child bodies, drops the
  sector's pairs that hold a body with the transient bit, then clears it.
  The body stays in the sector's lists.
- Every lift placement writes the body's three particle sets from its rest
  points and the placed matrix, its matrices and bounds, the node, and sets
  state |= 0x805 with the sleep countdown 0. So the release's raise
  (if lower, to 128) gives the item 128.
- The lock release clears 0x805, restores the environment, zeroes the
  contact mobility and freezes the node.
- The "is the space free" query (the AI's and the item cursor's marker
  test): the nine cells around the probe's mesh-box centre; each other body
  with a different root, not separated (bound sphere against mesh sphere),
  no conflicting layer channel, not disabled; the lower key first, the
  overlap-only body test, then the overlap pair test (each sphere in the
  other's frame: the box distance against r², then the centre distance
  against (r + R)², both passing a NaN, then the mesh's boolean sphere
  query at r); the first touch answers; then the probe's child bodies.

## What Changes

- exa_physics: `Ragdoll::hold_out`, `Ragdoll::place_grab_at`;
  `sector_lists::space_taken`, `overlap_touch`; `sector::static_overlap_meet`
  passes a NaN.
- exa_game: the lift holds the body out of the simulation and places it
  whole each frame; the lift's ray meets props with no stand-in (the held
  one left out); the release clears the disabled state; the marker overlap
  uses `space_taken`.

## Impact

A lifted item no longer moves by a spring: it sits where the cursor places
it, out of the simulation. Released, it is awake 128 ticks, and a neighbour
at its old place wakes to 128. Items can be set on dropped items. Story
levels and the duel are unchanged tick for tick.
