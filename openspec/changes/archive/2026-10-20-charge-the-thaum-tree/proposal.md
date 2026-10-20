# Proposal

## Why

The power tree's charge method advances a stored charge by the tick over 120, clamps it, derives a gain from either branch and stamps the first start. Exarust does not model it.

## What Changes

- The charge adds `tick / 120` in double precision and clamps to −9 through 5.
- A non-negative charge gives `(5 + charge) × 0.2`; a negative one gives `(4 − √−charge) × 0.25`.
- The started byte is set once and the clock stamped only on the first call.

## Capabilities

### New Capabilities

### Modified Capabilities

- `thaumaturgy`: the power tree's charge.

## Impact

- `exa_game::thaum`, from `charge_tree`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the 316-byte charge method and its 120/5/−9/0.2/4/0.25 constants. No new corpus files.
- Out of scope: the sound call it makes.

## Phase

Phase 5, thaumaturgy. The tree's charge.
