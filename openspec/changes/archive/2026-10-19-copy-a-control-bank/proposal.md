# Proposal

## Why

Clearing a session mark stores 0 and stops. The enter path also sets a flag and a code, then copies one of two banks of sixteen words.

## What Changes

- A focus that clears a mark which was not already 0 sets a flag to 0 and a code to 4.
- Side 0 copies the first bank when its flag is not 0, then clears that flag.
- Any other side raises a notice and copies the second bank when the flag is 0, then sets the flag.
- A flag that already matches leaves the words. The install after the copy is counted. Its body is not in.

## Capabilities

### New Capabilities

### Modified Capabilities

- `arena-career`: the control-bank copy after a session mark is cleared.

## Impact

- `exa_game::menu`, from `MatchGate::begin_match`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the enter path's flag, code, and sixteen-word copy. No new corpus files.
- Out of scope: the install body, a world update of the side, and the non-zero argument path.

## Phase

Phase 7, arena career. The bank copy, not the whole enter path.
