# Proposal

## Why

Phase 4 (AI) of docs/PLAN.md, backlog W1 "Door passage report". The door
bodies already follow the passage for the room route (the placed doors'
binding and the hinge each tick). The backlog assumed that the original
also feeds the passage into the navigation grid's door cells, so a door
opened in play becomes routable for NPC paths. This change traces that
report and wires what it does.

Evidence (REA, Ghidra listing of `Exanima.exe`): the door's init and its
hinged and sliding steps (their report blocks under the node's grid flag),
the global pathfinder's install and the level pathfinder's creation (it is
always the `TPathFinder` class), that class's and the abstract fallback's
method tables read from the image (the door slot of the first returns the
object itself; the second's is an empty call; no class derives from
`TPathFinder`), the readers of a node's navigation word (the node clone
and the character set-up only), and the callers of the grid's dirty-box
slots (the item cursor's lift and let-go only).

## What Changes

- Nothing reaches the grid from a door: the original's report writes the
  node's navigation word (4 while shut, held or toggling, the saved word
  otherwise) and calls a pathfinder slot that does nothing, so no cell
  changes. Exarust keeps no word and makes no call; the docs say why.
- `refresh_movers` reports whether a floor changed, and `nav::NavRevision`
  (Exarust's) counts the builds and the refreshes that change a floor.
- `--debug-path` plans again when the grid changes, so with
  `--nav-refresh-moved` its path goes through a door once the leaf rests
  open.

## Out of scope

- An NPC that walks through a door it did not see open (AI routing over the
  room links, C4/P7).
- Retail observation of an NPC at a door shut at load.

## Impact

`exa_world` (nav mover and view), `exa_game` (the path probe), `exarust`
(flag doc); docs/NAVIGATION.md, crates/exa_world/README.md,
crates/exa_physics/README.md and the door body docs.
