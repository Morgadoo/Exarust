# Proposal

## Why

Phase 2 closes and locks a door on a signal of 0. A signal of 1 does not yet open the doors under the node.

## What Changes

- A signalled open plays the unlock sound once when there is at least one door, and plays nothing when there are none.
- A latched or motorised door waits 120 ticks. A door that is neither does not wait.
- While the countdown is at least 2 it falls by 1. Below 2 the wait ends.

## Capabilities

### New Capabilities

### Modified Capabilities

- `object-physics`: the signalled open.

## Impact

- `exa_physics::door`.
- `docs/PLAN.md`, `docs/COMPONENT-MAP.md`.
- Evidence: the open of the doors under a node and the wait that precedes it. No format field changes, so the corpus step is a skip.
- Out of scope: the unlock sound's random pitch, and starting the open command when the wait ends.
