# Proposal

## Why

A full run slerps each bone 0.7 of the way from Walk to Run. The original uses the run blend itself while sprinting. That is the remaining piece of the Phase 3 run overlay (`docs/PLAN.md` §7, milestone M16).

## What Changes

- Shift with the cursor further than 3 m keeps the sprint latch. Shift with the walk keys does not: that stays a jog.
- While Walk plays, the overlay weight stays 0.7 times the run blend, and becomes the run blend itself when the latch is kept, the stance level is above 0.9 and the speed is above 3.5 cm a tick. The root stays the walk sample. The pose hold still follows the overlay.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `player-movement`: the sprint latch from Shift and a far cursor.
- `motion-playback`: the sprint overlay weight, and the level and speed gates.

## Impact

- `exa_game` steering records the latch. The living sampler picks the overlay weight.
- `docs/PLAN.md`, `docs/PLAYER.md` and `docs/formats/RSQ.md` (Motion playback).
- Evidence: the animation notes' walk/run blend and the movement notes' sprint latch, already recorded from the original. No format field changes, so the corpus step is a skip.
- Out of scope: the sprint gait row and its clock factor, the sprint run-blend cap, a gamepad sprint, changing how fast the body moves, playing Walk backwards, and gait plants.
