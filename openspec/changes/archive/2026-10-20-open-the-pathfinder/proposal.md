# Proposal

## Why

The pathfinder's constructor stores its cell size, depth, inverse, counter and seed and runs the base. Exarust does not model it.

## What Changes

- Cell size 30, depth 10, inverse 1/30, a zero counter and the given seed byte.
- The base runs once.

## Capabilities

### New Capabilities

### Modified Capabilities

- `navigation-grid`: the pathfinder's opening metrics.

## Impact

- `exa_world::nav`, from `pathfinder_metrics`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the 99-byte constructor's five stores and base call. No new corpus files.
- Out of scope: the base method.

## Phase

Phase 4, navigation. The pathfinder's metrics.
