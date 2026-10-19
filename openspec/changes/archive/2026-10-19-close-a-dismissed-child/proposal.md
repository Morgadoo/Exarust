# Proposal

## Why

A marked child is not only selected. The match begin closes it. An already closed child stops. Any other child closes its own children, then runs its close callback.

## What Changes

- Close a dismissed child with argument 0.
- An already closed child does not run its callback and does not close its children.
- A pinned child is marked closed and raises a notice, then still closes its children.
- Children close from the first to the last, before the parent's callback.

## Capabilities

### New Capabilities

### Modified Capabilities

- `arena-career`: the close a dismissed child runs.

## Impact

- `exa_game::menu`, from `MatchGate::begin_match`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the close's early return, the pinned byte, the child walk, and the callback. No new corpus files.
- Out of scope: the capture release, the task's other children, detaching, redraw, and the call after the two lists.

## Phase

Phase 7, arena career. One close, not the window class.
