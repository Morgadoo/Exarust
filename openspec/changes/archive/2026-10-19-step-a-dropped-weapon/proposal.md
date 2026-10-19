# Proposal

## Why

A dropped weapon keeps its release values and never enters the body solver, so it cannot fall.

## What Changes

- The dying tick puts a dropped weapon that has a solid shape into the body solver.
- The body is awake at 256, with the weapon's own mobility, friction 0.75, its spheres, and the velocity it already had.
- One solver tick moves it. A weapon with no solid shape spawns nothing. A second dying tick adds no body.

## Capabilities

### New Capabilities

### Modified Capabilities

- `physical-body`: the free body a dropped weapon becomes.

## Impact

- `exa_combat::collapse`, using the shipped rigid-body solver.
- `docs/PLAN.md`, `docs/COMPONENT-MAP.md`, and `docs/formats/OTHER.md`.
- Evidence: a dropped weapon is already a rigid body, its particle history is left alone, and a rigid body in the solver uses friction 0.75 and its mobility. No new corpus files.
- Out of scope: body-to-body contacts for the loose weapon, the flag mask that keeps bits other than 0, 16 and 17, and injuries beyond damage.

## Phase

Phase 4, the first duel. A dropped weapon as a free body.
