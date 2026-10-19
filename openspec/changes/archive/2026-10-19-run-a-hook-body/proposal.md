# Proposal

## Why

The gate hook and the watch hook are registered and never run.

## What Changes

- The gate hook clears two result words and sets the flags to 1. A set mid byte counts an alternate hook. The pick under the cursor is not in.
- The watch hook stores a stamp of 0 when bit 0x2000 is clear. With the bit, a ready target, and a held byte of 0, the stamp copies the clock. Code 1 clears bit 0x800000. Code 2 sets it. A pending count refreshes. A leave clears the focus and enters the session. The hook returns 1.

## Capabilities

### New Capabilities

### Modified Capabilities

- `arena-career`: the gate hook and the watch hook.

## Impact

- `exa_game::menu`, from `MatchGate::run_gate` and `MatchGate::run_watch`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the opening stores of the gate hook and the watch hook's flag, stamp, and leave. No new corpus files.
- Out of scope: the pick under the cursor, and the focus method that runs before the hand words.

## Phase

Phase 7, arena career. These two hooks, not every registered method.
