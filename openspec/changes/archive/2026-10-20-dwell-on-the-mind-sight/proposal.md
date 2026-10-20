# Proposal

## Why

The sight's tracker (`0x1001dfd60`) runs at most every four milliseconds and holds each node for a dwell window of 3000 or 5000 ms. Exarust has the weights and the quad but not the throttle or the dwell.

## What Changes

- A sight tick is due at least 4 ms after the last.
- The window is 3000 ms when the node's dwell value is above 1, 5000 ms when its body is marked, and none otherwise.
- The fade is the elapsed time over the window, and the node is lost once the clock passes `seen + window`.

## Capabilities

### New Capabilities

### Modified Capabilities

- `thaumaturgy`: the sight's dwell.

## Impact

- `exa_game::mind`, from `mind_track_due`, `mind_dwell_window` and `mind_dwell_fade`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`: `SG_Thaum_Mind` becomes **11%**. The published headline stays 14.4% and 5.7%; the fresh sum stays about 15.5%.
- Evidence: `0x1001dfd60`: the 4.0 ms throttle at `0x10032f290`, the `+0x11a4 > 1` 3000 ms window at `0x10032f2a8`, the `+0xdf4 & 0x800` 5000 ms window at `0x10032f2b0`, and the `now > seen + window` loss test. No new corpus files.
- Out of scope: the visibility test and the entry array.

## Phase

Phase 7, thaumaturgy. The sight's dwell.
