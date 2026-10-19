# Proposal

## Why

A banded form still has no square when the close mark is -4, after the two quads that join at the end of that band.

## What Changes

- On that band, a close mark of -4 draws one square from 80, 0 to 100, 20 at unit 5 and width 100.
- The earlier square remains.
- A mark of 0 or -3 draws none of this square.
- A style that is not that band, and a wide bit, draw none of it.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the close square drawn on the band after the two quads join.

## Impact

- `exa_ui::skin`, from `dispatch_form`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the close-mark compare on the join after those quads. No new corpus files.
- Out of scope: the call after that square, and the helper body on the clear-hold path.

## Phase

Phase 7, the interface. The close square, not the call after it.
