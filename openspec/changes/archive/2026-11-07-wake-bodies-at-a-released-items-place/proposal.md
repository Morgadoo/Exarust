## Why

PLAN Phase 3 (physics). The component gap lane for `TSimSector` and
`TTetraRBD` listed both components routine by routine. The sector's
placement overlap query and its per-pair placement test were ported as
tested functions in earlier rounds but had no caller: the item cursor's
release woke the item for the full count and nothing else.

## Evidence

REA (Exanima.exe), read against the assembly:
- The lift's release posts the put-back with the four particles the lift
  saved. The put-back raises the item's sleep countdown to 128 (half the
  full 256, only if lower), builds the item's matrix and bounds from the
  saved particles, asks the sector's placement overlap query unless the
  node has fallen out of the world, rebuilds the matrix from the current
  particles and places the item: 8 settling ticks, world contacts cleared,
  the out-of-world and asleep bits cleared, the placed bit set, the
  default environment back. Without saved particles (a drop handed to
  another view) the raise is to 31 and the query runs at the current pose.
- The query runs only in a grid sector. It walks the nine cells around the
  cell of the probe's mesh-box centre; each listed body passes the bound
  sphere, both oriented-box gates, the same-root group words or the layer
  channels, and the disabled bit, then the pair test with the lower order
  key first.
- The pair test takes each of the first body's collision spheres into the
  second's frame, pre-tests it against the second's mesh centre with
  reach `(r + 2) + R` and against its mesh box with `r + 2` (both compares
  skip only on an ordered "reach² below distance²", so a NaN passes), and
  asks the node mesh's sphere query at `r + 2`; the callback only marks a
  hit. On any hit the lower countdown is raised to the higher.

## What Changes

- exa_physics: `sector_lists::placement_touches`, `placement_touch`,
  `equalise_placement`, `place_released` and the release constants;
  `sector::placement_meet` lets a NaN through as the assembly does.
- exa_game: the lift's end (every outcome but the lock) wakes the item to
  at least 128, places it for 8 ticks, and posts the placement query at
  the frame the lift saved.

## Impact

Behaviour changes only when the item cursor lets go of an item: bodies
still touching the item's old place wake with it, a released item sleeps
after 128 quiet ticks instead of 256, and settles 8 ticks without inertia.
Scripted story levels and the arena duel are unchanged tick for tick.
