# Proposal

## Why

The pathfinder's A* prices every grid step with a fixed neighbour table, two per-cell penalty bytes and a 3D heuristic. Exarust has the line walk, footprint and plan helpers, but not the grid's coordinate and cost model.

## What Changes

- World to cell is `trunc(world × inverse unit + 0.5)`; cell to world is `(cell − 0.5) × unit`.
- The eight neighbours are NW, N, NE, W, E, SW, S, SE with costs 7, 5, 7, 5, 5, 7, 5, 7.
- A step needs a set floor, a rise of −20 through +4, A at most `180/(quality+1)` and shifted B at most 600.
- The step cost adds `10·dz²`, the direction cost, twice A and the shifted B; the heuristic is ten times the 3D distance, truncated.

## Capabilities

### New Capabilities

### Modified Capabilities

- `navigation-grid`: the path grid's coordinates and costs.

## Impact

- `exa_ai::path`, from `path_cell_coord`, `path_world_coord`, `path_step_ok`, `path_step_cost`, `path_heuristic`, `PATH_NEIGHBOURS` and `PATH_MOVE_COST`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`: `TPathFinder` becomes **32%**. The published headline stays 14.4% and 5.7%; the fresh sum stays about 15.5%.
- Evidence: the A* search: the `+0.5`, the `127.5`, the neighbour offsets and deltas, the costs `7,5,7,5,5,7,5,7`, the `−20/+4` rise, the `180` and `600` caps, the `10.0` distance scale and the `0.1` level scale. No new corpus files.
- Out of scope: the open/closed lists, the two-layer selection and the path reconstruction.

## Phase

Phase 6, AI. The path grid's costs.
