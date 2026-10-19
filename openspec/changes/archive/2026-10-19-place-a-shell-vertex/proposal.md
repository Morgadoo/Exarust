# Proposal

## Why

Phase 7 fades a shell ripple. The shell vertex and the strength of a hit are not placed.

## What Changes

- A shell vertex sits at its position plus 2 times the normalized sum of its bone. A zero sum adds nothing.
- A hit of 0.3 or more stores strength 6. A smaller hit stores (2 minus the scaled hit) times that scale times 6. The scale is the hit times 3.333.
- An eighth hit is refused.

## Capabilities

### New Capabilities

### Modified Capabilities

- `thaumaturgy`: the shell vertex and the hit strength.

## Impact

- `exa_game::thaum`, beside the shell ripple.
- `docs/formats/OTHER.md`, `docs/PLAN.md`, `docs/COMPONENT-MAP.md`.
- Evidence: the shell vertex offset and the hit curve. No new corpus files. Tests use synthetic points.
- Out of scope: copying the skin's index list, and the shell's stored size of 800.
