# Design

## Context

The planner is one function of the grid: a 256-cell window, nodes in a
fixed table, a LIFO open list scanned for the least total, a stamp table
per window place and layer, then a corner walk and string pulling with a
cost-bounded line walk. The refresh keeps working floors per cell, reset
from the built floors, raised by movers' vertices, and written back where
they differ. Docs: `docs/NAVIGATION.md` (Planning, Queries), no
`docs/formats/` page (no file format changes).

## Decisions

1. The earlier, unmerged Claude planner and dirty-rectangle work
   (wf_c6eec6b6) was the more faithful reading; it is the base, re-checked
   against the instructions and adapted to the current grid API.
2. The window, node and corner buffers live per thread; only the window's
   interior is cleared per search, as the original clears it. A grid
   smaller than the window reads as no floor outside it (the original
   reads past it).
3. `find_path` stops at the goal's cell (accept distance 10) and shifts
   nothing; `find_path_with` takes both, the shift masked to 31.
4. The line walk shares the planner's step rule. The two differ only where
   an upper floor is not above the lower one, which a built grid never
   holds.
5. The grid keeps a copy of its built floors; `set_cell` writes both (a
   grid written cell by cell counts as built), the refresh only the
   current floor.
6. The duplicate planner pieces in `exa_ai::path` are removed rather than
   corrected: `exa_world::nav` holds the planner and the walkable snap.

## Open questions

- The AI's default goal (size class 3, shift 1) comes from the earlier
  reading of the AI planner, outside this lane's evidence pack; the probe
  uses it and says so. Answer with REA on the AI planner's call when NPC
  planning is wired.
- What the search's carried take clears besides the carried entry (a slot
  of the character record) is unknown; answer with REA on the character
  record's fields.
- The corpse's grip is taken as the main hand; confirm the held part with
  REA on the character's hand parts.

## Risks

- Removing `exa_ai::path`'s tail may conflict with lanes appending there.
