# Proposal

## Why

The pathfinder clears its 256×256 stamp table to the blocked value before each search: the first 256 entries, the tail from `0xfeff`, and both edge entries of rows 1 through 254. Exarust did not model this border ring.

## What Changes

- The clear fills the first 256 entries and `0xfeff` through `0xffff` with the blocked value.
- It fills the first and last entry of rows 1 through 254.
- The interior is left to the caller; a short table is ignored.

## Capabilities

### New Capabilities

### Modified Capabilities

- `navigation-grid`: the stamp clear.

## Impact

- `exa_ai::path`, from `PATH_STAMP_SIDE`, `PATH_STAMP_BLOCKED` and `clear_path_stamps`.
- `docs/PLAN.md` and the generated `docs/COMPONENT-MAP.md`: `TPathFinder` becomes **64%** in `data.json`. The headline stays 17.0%.
- Evidence: the stamp clear: the three fill loops and the `0xffffffff` constant. No new corpus files.
- Out of scope: the constructor's globals.

## Phase

Phase 6, AI. The stamp clear.
