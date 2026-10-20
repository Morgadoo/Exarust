# Proposal

## Why

The mind-sight tracker (`0x1001dfd60`) weights each visible node from six body values and blends the entry toward them by `2^(−dt/1000)`. Exarust has the overlay fade and pick but not the weights.

## What Changes

- `A = 0.2·v78` and `B = 0.2·(v974 + v978 − v7b4)`; a marked body or `A > 1` forces all three weights to 1.
- Otherwise `A` is floored at 0, `B` clamped to 0..1, the maximum their larger, the ratio `A/(A+B)`, and the threshold the largest of `maximum + 0.2`, `0.6·v7c` and `0.015·v74`, capped at 1.
- The entry blends toward `(threshold, maximum, ratio)` by `2^(−dt/1000)`, or one half for a step of a second or more.

## Capabilities

### New Capabilities

### Modified Capabilities

- `thaumaturgy`: the sight's node weights.

## Impact

- `exa_game::mind`, from `MindBodyValues`, `MindWeights`, `MindEntryColors`, `mind_track_weights` and `blend_mind_weights`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`: `SG_Thaum_Mind` becomes **7%**. The published headline stays 14.4% and 5.7%; the fresh sum stays about 15.5%.
- Evidence: `0x1001dfd60` (the tracker) and `0x1001dfaf0` (the weights): the 0.2 factor at `0x10032f270`, the 0.6 and 0.015 floors at `0x10032f278`/`0x10032f288`, the 1.0 clamp, the marked `+0xdf4 & 0x800` test and the `2^(−dt/1000)` decay. No new corpus files.
- Out of scope: the dwell windows and visibility tests.

## Phase

Phase 7, thaumaturgy. The sight's node weights.
