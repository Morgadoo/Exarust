# Proposal

## Why

The cloth's body-collision pass routes each particle through volume and segment tests, calls the body's collision callback and pushes the particle to the body's radius surface. Exarust does not model it.

## What Changes

- A cloth tested against its own body returns unchanged.
- A particle without a volume hit or without both segment acceptances is unchanged and makes no callback.
- An accepted hit makes one callback, stores its index and side.
- A resolved hit snaps the particle to the edge point when it is inside the radius, else moves it to the radius along the edge.

## Capabilities

### New Capabilities

### Modified Capabilities

- `cloth-step`: the cloth's body collision.

## Impact

- `exa_physics::cloth`, from `collide_cloth`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the 780-byte method's two volume branches and its radius correction. No new corpus files.
- Out of scope: the volume and segment queries and the callback body.

## Phase

Phase 3, bodies and cloth. The cloth's body collision.
