# Proposal

## Why

Phase 2 steps a lever and a pressure plate. A signal from either does not yet choose a door, a fire, a light, a relay or a plate.

## What Changes

- A missing target does nothing. A door closes on 0 and opens on any other value. −1 toggles: an unlatched leaf closes and a latched leaf opens. An inverted door flips that 0 and 1. −1 still toggles. A door wins over an operative record on the same object.
- A fire job starts, at its stored size, only when the value is not 0. A light job sets the stored bit to the value's low bit. A relay of kind 0, 1, 2, 3 or 5 is named. Any other kind is ignored. A plate is pushed exactly when the value is not 0.

## Capabilities

### New Capabilities

### Modified Capabilities

- `object-physics`: where a signal goes.

## Impact

- `exa_physics::signal`.
- `docs/PLAN.md`, `docs/COMPONENT-MAP.md`.
- Evidence: the signal dispatch, the deferred job, and the toggle of the doors under a node. No format field changes, so the corpus step is a skip.
- Out of scope: the relay bodies, and carrying out the close.
