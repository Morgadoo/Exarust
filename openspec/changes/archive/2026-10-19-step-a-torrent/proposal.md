# Proposal

## Why

Phase 7 steps a wave front and a shell ripple. A torrent's level and its two channels are not stepped.

## What Changes

- Below phase 1, the level is (2 minus the charge) times the charge times 0.4.
- Otherwise the level falls by 0.01, and below 0.02 it stops.
- Phase 1 eases the two channels toward 0.5 and 0.6 by 0.06.
- Phase 2 multiplies both channels by 0.96.

## Capabilities

### New Capabilities

### Modified Capabilities

- `thaumaturgy`: the torrent level and its two channels.

## Impact

- `exa_game::thaum`, beside the wave front.
- `docs/formats/OTHER.md`, `docs/PLAN.md`, `docs/COMPONENT-MAP.md`.
- Evidence: the torrent mix step. No new corpus files. Tests use synthetic charges.
- Out of scope: the stream mesh, the pushes, and the torrent's tick rate.
