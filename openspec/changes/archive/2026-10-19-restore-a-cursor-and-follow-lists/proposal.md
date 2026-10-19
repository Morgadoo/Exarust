# Proposal

## Why

Releasing a capture does not yet publish the capture underneath it. The two window lists also return without the call that follows them.

## What Changes

- When a capture remains, publish its code and name. Bit 0x8000 passes its low byte as the shape. Any other code passes 0 and marks the default.
- An empty capture list publishes code 0 and no holder, and does not mark the default.
- After the two lists, argument 1 runs when a focus is set or the mask shares bit 0. The mode word keeps bit 1 and sets bit 0. The stamp copies the clock. A ready hand list copies the ten stored words. A focus also latches and clears the pending byte.

## Capabilities

### New Capabilities

### Modified Capabilities

- `arena-career`: the cursor restored by a capture release, and the call after the two lists.

## Impact

- `exa_game::menu`, from `MatchGate::begin_match`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the remaining capture record, and the ten words copied after the lists. No new corpus files.
- Out of scope: the shape table, and the virtual call on the focus path.

## Phase

Phase 7, arena career. The restore and the follow call, not the whole cursor.
