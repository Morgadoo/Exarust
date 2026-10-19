# Proposal

## Why

The cloth's sync method skips its transform when the object's meta flag 0x10 is set and otherwise transforms and calls the pose with scale 1. Exarust does not model it.

## What Changes

- A set meta bit 0x10 skips the transform and the pose call.
- A clear bit transforms once and calls the pose with 1.0 twice.

## Capabilities

### New Capabilities

### Modified Capabilities

- `cloth-step`: the cloth's sync.

## Impact

- `exa_physics::cloth`, from `sync_cloth`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the 108-byte cloth sync's guard and two calls. No new corpus files.
- Out of scope: the transform and pose bodies.

## Phase

Phase 3, bodies and cloth. The cloth's sync.
