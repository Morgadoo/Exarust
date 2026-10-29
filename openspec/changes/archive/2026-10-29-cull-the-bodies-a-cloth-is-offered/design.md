## Context

The cloth stage (`exa_world::cloth`) gathered every part of every body once a tick and, for each cloth, filtered and copied the whole list into the cloth's body stage. With fifteen awake hair cloths on exanima01 (6 023 parts) that was about 108 300 offered parts and copies a tick. The sector routines are described in `crates/exa_physics/README.md` (sector section); there is no `docs/formats/` page for the runtime sector.

## Decisions

- **The nine-cell walk is the original's rule.** In the grid sector step, the stage that hands bodies to simulated objects with the body class reads the object's cell index and, for each of the nine entries of the sector's neighbour offset table, walks that cell of the body grid. A body is linked in one cell, the cell of its mesh box's world centre, re-bucketed every tick after the iterations. Exarust reproduces this with `sector::CellGrid::around` over a cell grid of the level's parts rebuilt once a tick in the cloth stage (`O(parts)` a tick, then `O(parts in nine cells)` per cloth).
- **No contact is lost.** A part the pre-test passes has its bound sphere meeting the object's; with both radii under a cell (a hair's bound is well under 1 m, a body part's too) their centres' cells differ by at most one on each axis, so the walk only drops parts the pre-test would reject. The `bodies` counter is therefore unchanged by this change; only `parts` falls.
- **The object list's origin.** The object grid's index includes the origin; an origin of −2048 made every index negative, so the overflow guard parked every object at `columns + 1`. With +2048 an object's bucketed cell is its own and the walk starts there. The level's static grid uses cells only, so its origin is left alone.
- **The hair cloth default.** With the offer bounded by the nine cells, an awake hair costs the pre-test of the parts near it (as one more awake body's broad phase), the contacts with the parts its bound reaches (its own head and neck, anyone touching it) and its level stage; none of these grows with the level. The default goes back on; `EXA_HAIR_CLOTH=0` turns it off.

## Open questions

- The order of the neighbour offset table is the one the earlier sector lane recorded (`NEAR_OFFSETS`); its writer is outside the sector pack [answer with REA: xrefs to the table's slot in the sector's grid setup].
- In-cell order: newest-first in the original; it only changes the order the particle contact visits bodies within a cell.
