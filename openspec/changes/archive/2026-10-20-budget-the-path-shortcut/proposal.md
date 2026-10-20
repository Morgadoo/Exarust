# Proposal

## Why

The pathfinder's reconstruction line-walk is given a budget of `g[later] − g[kept] + 10`. Exarust's shortcut takes an unbudgeted predicate.

## What Changes

- A budgeted shortcut passes the clear predicate `cost[later] − cost[kept] + 10`.
- It shortens like the unbudgeted shortcut otherwise, keeping the first and last points.

## Capabilities

### New Capabilities

### Modified Capabilities

- `navigation-grid`: the budgeted shortcut.

## Impact

- `exa_ai::path`, from `shortcut_path_with_budget`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`: `TPathFinder` becomes **61%**. The published headline stays 14.4% and 5.7%; the fresh sum stays about 15.5%.
- Evidence: `0x1000c59a0`: the budget `g[i] − g[prev] + 10` at `0x1000c6c97` passed to the line walk `0x1000c76e0`. No new corpus files.
- Out of scope: the two-layer floor rule.

## Phase

Phase 6, AI. The budgeted shortcut.
