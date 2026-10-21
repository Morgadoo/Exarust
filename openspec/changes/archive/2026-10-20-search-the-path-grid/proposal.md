# Proposal

## Why

The pathfinder's A* is the component's core: a bounded best-first search with a LIFO open list popped by the smallest `f`, a 4095-node cap, an acceptance distance and a parent-chain path. Exarust has the grid's coordinates and costs but not the search.

## What Changes

- The search starts at the start cell, pushes it onto a LIFO open list and pops the strict minimum `f` (the newest wins a tie).
- It stops on the acceptance distance, an empty list or 4095 nodes.
- A neighbour enters when the step is accepted; a smaller `f` re-parents an open node.
- The path is the parent chain from the best node, collapsed through collinear runs.
- The goal counts as reached only when the best node is the goal cell.

## Capabilities

### New Capabilities

### Modified Capabilities

- `navigation-grid`: the A* search.

## Impact

- `exa_ai::path`, from `PathGrid`, `PathRequest`, `PathSearch` and `search_path`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`: `TPathFinder` becomes **44%**. The published headline stays 14.4% and 5.7%; the fresh sum stays about 15.5%.
- Evidence: the A* search: the 16-byte nodes, the `g` and `f` fields, the LIFO open head and linear strict-minimum pop, the `0xfff` cap, the query's acceptance distance, the re-parent test on `f`, the parent walk with collinear collapse, and the `A/3` seed pseudo-height. No new corpus files.
- Out of scope: the two-layer floor selection, the line-walk shortcut and the endpoint penalty fix-up.

## Phase

Phase 6, AI. The path grid's search.
