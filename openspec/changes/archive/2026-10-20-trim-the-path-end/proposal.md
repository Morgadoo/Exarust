# Proposal

## Why

The pathfinder's reconstruction ends with a fix-up that drops up to eight path points whose penalty byte is above `64/(quality+1)`. Exarust has the A* and the shortcut but not this trim.

## What Changes

- From the end, up to eight points with a penalty above `64/(quality+1)` are dropped.
- The first point is always kept.
- The remaining count is returned.

## Capabilities

### New Capabilities

### Modified Capabilities

- `navigation-grid`: the endpoint fix-up.

## Impact

- `exa_ai::path`, from `trim_path_end`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`: `TPathFinder` becomes **60%**. The published headline stays 14.4% and 5.7%; the fresh sum stays about 15.5%.
- Evidence: the A* search's endpoint walk of at most eight parents while a node's penalty is above `64/(quality+1)`. No new corpus files.
- Out of scope: the greedy line-walk budget inside the same reconstruction.

## Phase

Phase 6, AI. The endpoint fix-up.
