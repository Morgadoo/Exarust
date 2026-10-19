# Proposal

## Why

Phase 7 steps a blast charge and a barrier ripple. A wave's charge rate and its front are not stepped.

## What Changes

- A wave rate is the tick in seconds, then five thirds of that.
- While charging or fizzling, the radius grows by 0.0015.
- Once released, the reach grows by 0.6, the radius by 0.0006 of the speed, and the trail step is 0.07 of the speed plus 1.
- An amount of 0.99 releases on a 12 ms wave rate. The blast rate for that tick leaves it at 1.

## Capabilities

### New Capabilities

### Modified Capabilities

- `thaumaturgy`: the wave charge and the released front.

## Impact

- `exa_game::thaum`, beside the blast charge.
- `docs/formats/OTHER.md`, `docs/PLAN.md`, `docs/COMPONENT-MAP.md`.
- Evidence: the wave rate scale and the front's growth constants. No new corpus files. Tests use synthetic ticks.
- Out of scope: the trail ring, the wave mesh, torrent and shell.
