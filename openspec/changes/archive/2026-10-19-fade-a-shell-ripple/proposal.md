# Proposal

## Why

Phase 7 steps a barrier ripple. A shell ripple fades by its own scale, and that step is not in.

## What Changes

- A shell ripple loses 0.04 times its scale from the strength.
- The shown value moves toward that strength by at most 0.06 times the scale upward and 0.04 times the scale downward.
- Below 0.05 on both, the ripple is gone.

## Capabilities

### New Capabilities

### Modified Capabilities

- `thaumaturgy`: the shell ripple.

## Impact

- `exa_game::thaum`, beside the barrier ripple.
- `docs/formats/OTHER.md`, `docs/PLAN.md`, `docs/COMPONENT-MAP.md`.
- Evidence: the shell ripple step and its stored scale of 1. No new corpus files. Tests use synthetic ripples.
- Out of scope: the copied mesh, torrent, and the shell's 800 cm size.
