# Proposal

## Why

The walkable-cell snap (4,829 bytes) moves a point that is not on a walkable cell onto the nearest one: it accepts the start when its penalties are low, otherwise sweeps a 32×32 window in distance order up to 220 nodes. Exarust has the A* but not this snap.

## What Changes

- A cell is walkable when it has a floor and its A and B bytes sum under 80.
- A walkable start returns at once, with its height = base + floor·10.
- Otherwise a best-first 32×32 sweep in distance order, up to 220 nodes, returns the nearest walkable cell or nothing.

## Capabilities

### New Capabilities

### Modified Capabilities

- `navigation-grid`: the walkable snap.

## Impact

- `exa_ai::path`, from `SnapGrid`, `SnapResult` and `snap_to_walkable`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`: `TPathFinder` becomes **58%**. The published headline stays 14.4% and 5.7%; the fresh sum stays about 15.5%.
- Evidence: the snap: the world-to-cell map, the `A+B < 80` acceptance, the 32×32 clear block, the best-first distance pop (`10·√(dx²+dy²+dz²)`), the `0xdc` = 220 node cap, and the cell-centre height `base + floor·10` with the `127.5` bias. No new corpus files.
- Out of scope: the two entry variants and the layer choice.

## Phase

Phase 6, AI. The walkable snap.
