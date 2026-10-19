# Proposal

## Why

Phase 2 broadcasts a signal that asks a door to close. Close-and-lock does not yet drive the leaf shut or bar it.

## What Changes

- A motorised hinge sets its fraction to 1 and drives toward shut for 300 ticks, or 100 when fast. A hinge that is not motorised does not start that drive.
- A sliding leaf that is not latched plays the lock sound. A fast sliding leaf drives toward shut for 100 ticks.
- A leaf that is already shut becomes latched.

## Capabilities

### New Capabilities

### Modified Capabilities

- `object-physics`: close-and-lock.

## Impact

- `exa_physics::door`.
- `docs/PLAN.md`, `docs/COMPONENT-MAP.md`.
- Evidence: the close-and-lock of the doors under a node. No format field changes, so the corpus step is a skip.
- Out of scope: the lock sound's random pitch.
