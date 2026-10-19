# Proposal

## Why

Phase 7 casts a blast as one knock, but the blast's charge, the chunk growth and the separation of two chunks are not stepped.

## What Changes

- The charge rate is five sixths of the tick in seconds.
- Charging rises by the rate and releases when it passes 1. Fizzling falls by twice the rate. After release the amount rises by three times the rate.
- A chunk grows by the tick times 0.7 of its speed plus 3, times 0.0006.
- Two chunks closer than twice the first radius move apart by half the overlap.

## Capabilities

### New Capabilities

### Modified Capabilities

- `thaumaturgy`: the blast charge and the two chunk motions.

## Impact

- `exa_game::thaum`, beside the blast cast.
- `docs/formats/OTHER.md`, `docs/PLAN.md`, `docs/COMPONENT-MAP.md`.
- Evidence: the blast start and the blast step. No new corpus files. Tests use synthetic amounts and points.
- Out of scope: wave, torrent, barrier and shell meshes.
