# Proposal

## Why

Closing a dismissed child still skips the capture release, the task's other children, the hover clear, and the redraw. Those steps run in the same close.

## What Changes

- Release a capture, and clear style bit 0x20, when this child holds it. Do that even if the child is already closed.
- Clear a hover on this child. Close the task's other children that name it, first to last, before its own children.
- Hide a shown child. Ask for a redraw, whose word gains bits 0 and 1, and ask again after the callback.
- When the child is the active form, clear that pointer and skip the hover, the siblings, and the redraw.

## Capabilities

### New Capabilities

### Modified Capabilities

- `arena-career`: the rest of the close a dismissed child runs.

## Impact

- `exa_game::menu`, from `MatchGate::begin_match`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the capture bit, the hover clear, the sibling walk, and the redraw word. No new corpus files.
- Out of scope: restoring a cursor from a remaining capture, and the call after the two lists.

## Phase

Phase 7, arena career. The rest of one close, not the window class.
