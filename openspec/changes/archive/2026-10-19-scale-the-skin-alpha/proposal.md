# Proposal

## Why

The parent skin stores two alpha bytes and writes their scaled values across the colour ramps. Exarust does not model that rescale.

## What Changes

- The first stored alpha, scaled by the horizontal scale, is written to 14 ramp bytes.
- The second stored alpha, scaled by the vertical scale, is written to 48 ramp bytes.
- Each product rounds to nearest even and keeps the low byte.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the skin's scaled ramp alpha.

## Impact

- `exa_ui::skin`, from `scale_skin_alpha`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the parent scale method's two byte reads and its 62 stores. No new corpus files.
- Out of scope: the ramps that read those bytes.

## Phase

Phase 7, the interface. The alpha, not the ramp colours.
