# Proposal

## Why

Two Mind powers are small control hooks: the mind mask sets or clears body bit `0x10000`, and the mind sense triggers while it is being removed. Exarust now has the spirit form but not these toggles.

## What Changes

- The mind mask starts removal on event 2; while removing it clears the bit and is inactive, otherwise it sets the bit and is active.
- The mind sense starts removal on event 2; while removing it is active and triggers only with a stored value above zero; otherwise it is inactive and does not trigger.

## Capabilities

### New Capabilities

### Modified Capabilities

- `thaumaturgy`: the mind mask and sense toggles.

## Impact

- `exa_game::mind`, from `MindEffect`, `MindMaskStep`, `MindSenseStep`, `step_mind_mask` and `step_mind_sense`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`: `SG_Thaum_Mind` becomes **3%**. The published headline stays 14.4% and 5.7%; the fresh sum stays about 15.5%.
- Evidence: the two mind toggles: one (event 2 → state 2, clear/set `0x10000` on the body's flag word and return 2/0) and the other (event 2 → state 2, its value against 0, and the return 2/0). No new corpus files.
- Out of scope: the mask's visual and the sense's target scan.

## Phase

Phase 7, thaumaturgy. The mind effects.
