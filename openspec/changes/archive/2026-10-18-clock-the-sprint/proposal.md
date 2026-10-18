# Proposal

## Why

A sprint still plays the jog's clip clock. The original blends the forward clock from the walk rate toward the feral sprint row, then slows it by encumbrance. That is the next piece of the Phase 3 gait (`docs/PLAN.md` §7, milestone M16), after the sprint overlay weight.

## What Changes

- While sprinting, a forward gait's clock blends from the walk rate toward the feral sprint row by the square root of the run blend, then multiplies by 1 − 0.1 times the encumbrance. The feral sprint row is the sprint rate blended toward the jog rate by the feral level.
- A jog, a backpedal and a sprint that has not passed the level and speed gates keep the clocks they have. The capsule still moves at the jog speed.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `motion-playback`: the sprint clock on a forward gait.

## Impact

- `exa_game` living sampler reads the feral level and advances the forward clock on the feral sprint row while sprinting.
- `docs/PLAN.md`, `docs/PLAYER.md` and `docs/formats/RSQ.md` (Motion playback).
- Evidence: the animation notes' locomotion clock and the movement notes' gait rows, already recorded from the original, including the feral blend of the sprint row. No format field changes, so the corpus step is a skip.
- Out of scope: the sprint row's thrust, the tighter run cap, the height and crouch clock factors, a linear run blend, playing Walk backwards, gait plants, and a gamepad sprint.
