## Why

With long hair as cloth on, exanima01 ran at 54–59 FPS on the RTX 3090 instead of 60: the cloth stage's `parts` split was about 1.6 ms a tick, bucketing every part of the level's ~4 800 bodies by 6 m cell every tick, plus a map of every part's countdown built every tick for the hair's owner lookup. Nearly all of those bodies sleep and do not move, and the cloths read only the cells around themselves.

## What Changes

- The cloth stage keeps an incremental cell index of the body parts (`exa_world::cloth_parts::PartCells`): each tick it walks the bodies, re-buckets only those whose `Ragdoll` changed since the last tick (a deep sleeper is left untouched by the body stage, as its drawing and prop stand-ins already rely on), drops the bodies that are gone, and nothing at all while no cloth is simulated.
- The stage gathers only the parts linked in the nine cells around each cloth, each cell's in the order a full re-bucket would link them (the bodies' walk order, then part order), so every cloth is offered exactly what it was offered before; the particles are unchanged bit for bit.
- A hair cloth's owner is found through the index's part-to-body lookup instead of a per-tick map of every part.
- `EXA_DEBUG_CLOTH=1` reports the bodies re-bucketed and the parts gathered with the `parts` split.
- Long hair hangs as cloth by default; `EXA_HAIR_CLOTH=0` keeps the styles rigid.
- A CPU bench of fifteen hairs among 4 800 sleeping bodies of ten parts (exanima01's count).

Phase 3 (bodies: skeleton, physics, control) in docs/PLAN.md, the cloth stage of the bodies' tick and the hair cloth switch.

Evidence: the original's grid sector re-buckets its bodies every tick and walks the nine cells around each simulated object, masking out a body's sleep bit only when the object itself sleeps, so an awake object is offered sleeping bodies and they cannot be skipped; the index gives the same cells as a full re-bucket. The level bench's parts split fell from 3.2 to 0.08 ms a tick on the build worker with the same particle checksum.

## Out of scope

- Any change to which parts a cloth is offered, the cloth simulation itself, or the object list.
- Costs of the hair outside the fixed schedule (drawing, transform propagation).
