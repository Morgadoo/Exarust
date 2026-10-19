# Proposal

## Why

Phase 2 steps a lever toward its detent. A pressure plate does not yet travel, grind, or click.

## What Changes

- A pressed plate raises each travel by 0.02, at most 1. A free plate lowers each by 0.01, at least 0. A one-shot stops being pressed once a travel passes 1, and later travels that tick fall. The mean is written back to every travel.
- The grind speed is 0.9 times the mean's change plus 0.1 times the old speed. It stops at 0.001 or below. Otherwise the volume is 50 times the speed, squared, and the pitch is 20 times the speed plus 0.8, at most 1.
- A mean above 0.7 presses a plate that was up, clicks, and signals 1, or −1 when inverted. A mean below 0.5 releases a plate that was down and signals 0, unless inverted. A travel of 0.68 that rises does not press. A travel of 0.51 that falls does not release.

## Capabilities

### New Capabilities

### Modified Capabilities

- `object-physics`: the pressure plate's step.

## Impact

- `exa_physics::plate`.
- `docs/PLAN.md`, `docs/COMPONENT-MAP.md`.
- Evidence: the pressure-plate step. No format field changes, so the corpus step is a skip.
- Out of scope: a body standing on the plate, and the click's random pitch.
