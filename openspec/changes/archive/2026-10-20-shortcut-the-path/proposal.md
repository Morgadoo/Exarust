# Proposal

## Why

The A* returns a cell path that is then string-pulled: a point is dropped when the direct line from the last kept point to the following one is clear. Exarust has the search but not the shortcut.

## What Changes

- The first point is always kept.
- From the last kept point, a point is dropped when the direct line to the following point is clear, and kept otherwise.
- The last point is always kept.

## Capabilities

### New Capabilities

### Modified Capabilities

- `navigation-grid`: the path shortcut.

## Impact

- `exa_ai::path`, from `shortcut_path`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`: `TPathFinder` becomes **59%**. The published headline stays 14.4% and 5.7%; the fresh sum stays about 15.5%.
- Evidence: the A* search's reconstruction: the backward line-walk test with the `dg + 10` budget, the kept bends and the first and last points. No new corpus files.
- Out of scope: the original's exact backward indexing and its endpoint penalty fix-up.

## Phase

Phase 6, AI. The path shortcut.
