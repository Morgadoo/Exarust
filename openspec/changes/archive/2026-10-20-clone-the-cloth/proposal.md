# Proposal

## Why

The cloth's clone method creates a new cloth and copies four fields from the original. Exarust does not model it.

## What Changes

- A new cloth is created.
- Fields 0x20, 0x140, 0x144 and 0x148 copy unchanged.

## Capabilities

### New Capabilities

### Modified Capabilities

- `cloth-step`: the cloth's clone.

## Impact

- `exa_physics::cloth`, from `clone_cloth`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the 99-byte clone's four copies. No new corpus files.
- Out of scope: the allocation.

## Phase

Phase 3, bodies and cloth. The cloth's clone.
