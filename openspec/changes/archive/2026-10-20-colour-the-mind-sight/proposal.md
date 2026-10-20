# Proposal

## Why

The sight's draw colours each entry from its three blended values: two fixed colours, a third lerped by the last value, two darkened alphas and a quad alpha reduced by the entry's fade. A kind-1 entry uses a pale palette. Exarust has the quad basis but not the colours.

## What Changes

- colour0 is {0.1, 0.4, 0.6}, colour1 {0.8, 0.6, 0}, colour2 lerped from {0.4, 0, 0} to {0.31, 0.78, 0} by the third value.
- The alphas are `0.1 + 0.6·(1 − first)` and `0.1 + 0.6·(1 − second)` in double precision; the quad alpha is the overlay alpha less a positive entry fade.
- A kind-1 entry uses {0.3, 0.3, 0.3} and {0.5, 0.5, 0.6} with alphas 1 and 0.5.

## Capabilities

### New Capabilities

### Modified Capabilities

- `thaumaturgy`: the sight's entry colours.

## Impact

- `exa_game::mind`, from `MindEntryStyle`, `MindColors` and `mind_entry_colors`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`: `SG_Thaum_Mind` becomes **10%**. The published headline stays 14.4% and 5.7%; the fresh sum stays about 15.5%.
- Evidence: `0x1001e01d0`: the fixed colours at `0x1002e88b0`/`c0`, the third lerp at `0x1002e88d0`/`e0`, the `0.1 + 0.6·(1 − value)` alphas at `0x1001e0149` in double precision, the kind-1 constants at `0x1002e88f0`/`0x1002e8900`, and the per-entry fade subtraction at `0x1001e0633`. No new corpus files.
- Out of scope: the shader upload.

## Phase

Phase 7, thaumaturgy. The sight's colours.
