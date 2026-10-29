## Why

With the hair cloth on, exanima01 ran at 16 FPS on the RTX 3090: the living characters' hair never sleeps (a hair cloth stays awake on its head's countdown, as the original's), and the cloth stage offered each awake cloth every body part of the level (about 108 300 a tick for 15 awake cloths), so the stage cost about 8 ms a tick. The original's grid sector offers a simulated object only the bodies linked in the nine body-grid cells around the object's own cell; the change `gate-the-hair-cloth` turned the hair cloth off until the stage did the same.

## What Changes

- The cloth stage buckets the level's body parts once a tick by the 6 m grid cell their mesh box's world centre falls in, as the sector's body grid links them, and offers each cloth only the parts in the nine cells around the cloth's own cell (own cell first, then the neighbour table's order), before the sleep filter and the object pre-test.
- The cloth object list's origin cell puts the world origin in the grid's middle, so an object near the origin is bucketed in its own cell instead of being parked by the overflow guard.
- Long hair hangs as cloth by default again; `EXA_HAIR_CLOTH=0` leaves it rigid.

Phase 3 (bodies) in docs/PLAN.md: the sector's simulated-object stages and the hair cloth.

## Evidence

REA/Ghidra on Exanima.exe: the grid sector step's object-against-bodies stage walks the body grid's nine cells around the object's cell index (the neighbour offset table, each cell's linked list) with the bound-sphere test and the object-centre-to-mesh-box test, skipping disabled bodies and, for a sleeping object, sleeping ones; the particle-mode body contact gives the owner's spheres no padding and any other body 1 cm. The small-scene all-pairs sector offers every body instead (not used by the levels). GPU check: `EXA_DEBUG_CLOTH=1 play exanima01 --perf-log 5`.

## Out of scope

- The all-pairs sector variant (its object stage offers every body with the roles of the pre-test swapped).
- The order of the bodies within one cell (the original's list is newest first; Exarust's cell keeps the order of the level's body list).
- The cost of the cloth's own step and level stage.
