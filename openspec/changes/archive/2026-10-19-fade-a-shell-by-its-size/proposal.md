# Proposal

## Why

Phase 7 counts a skinned shell's index room. The stored size of 800, and the fade that divides by it, are not in.

## What Changes

- A shell stores a size of 800.
- While opening, the shown value is the elapsed time divided by that size times 1.5. Above 1 it becomes 1 and leaves that state, or enters the hold state when held.
- The hold state moves on. Its stamp is the clock minus the size times the remainder, rounded to the nearest even integer.
- While closing, the shown value is 1 minus the elapsed time divided by the size. A value that is not above 0 becomes 0 and finishes. A held close stores 3 times that value and then shows 1.
- Any other state stays as it is.

## Capabilities

### New Capabilities

### Modified Capabilities

- `thaumaturgy`: the shell fade that uses the stored size of 800.

## Impact

- `exa_game::thaum`, beside the shell ripple.
- `docs/formats/OTHER.md`, `docs/PLAN.md`, `docs/COMPONENT-MAP.md`.
- Evidence: the stored size and the fade that divides by it. No new corpus files. Tests use synthetic clocks.
- Out of scope: the shell light, and uploading the GPU buffers.
