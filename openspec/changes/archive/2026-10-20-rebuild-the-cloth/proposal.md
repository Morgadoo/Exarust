# Proposal

## Why

The cloth's rebuild copies its origin, rejects a mismatched override, transforms every particle point into a fresh buffer and hands it to the body. Exarust does not model it.

## What Changes

- A cloth that is not its own root returns without work.
- A root copies the origin pointer at 0xf0.
- A present override that differs from the origin also returns.
- Otherwise every particle point is transformed into a new buffer and the buffer is submitted.

## Capabilities

### New Capabilities

### Modified Capabilities

- `cloth-step`: the cloth's rebuild.

## Impact

- `exa_physics::cloth`, from `rebuild_cloth`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the 224-byte cloth rebuild's three guards and its per-particle transform. No new corpus files.
- Out of scope: the transform and submission bodies.

## Phase

Phase 3, bodies and cloth. The cloth's rebuild.
