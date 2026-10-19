# Proposal

## Why

A later body replaced the hit even when it sat past the ray, and a new pane did not move its children.

## What Changes

- The ray starts at length 4000 along z. A centre past that length misses. A closer hit shortens the ray, so a farther centre is left.
- A stretch child grows by the pane's change. An anchor child moves by that change. A centre child is placed at half the spare.

## Capabilities

### New Capabilities

### Modified Capabilities

- `arena-career`: the ray length and the pane's children.

## Impact

- `exa_game::menu`, from `MatchGate::run_gate`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the segment test against the ray, the shortened length after a hit, and the child flags in the size call. No new corpus files.
- Out of scope: nested children, and a ray that is not along z.

## Phase

Phase 7, arena career. This length and these children, not a free camera ray.
