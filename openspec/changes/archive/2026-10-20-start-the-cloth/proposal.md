# Proposal

## Why

The cloth's start method seeds its fields, installs an identity matrix, derives its step from the particle count and links a flagged body. Its frame method rotates three vertex buffers or copies the pending one. Exarust models the cloth step but not these two methods.

## What Changes

- Start sets flag 0x10000, copies the seed, installs a 4×4 identity, and takes the square root of the count times 1e-6 as its step.
- A body whose flags carry 0x10000000 sets the link flag and is stored.
- Frame with at most one pending makes two copies from the source, clears the pending count and sets flag 1.
- More pending rotates the three buffers and stores the frame index.

## Capabilities

### New Capabilities

### Modified Capabilities

- `cloth-step`: the cloth's start and frame swap.

## Impact

- `exa_physics::cloth`, from `start_cloth` and `submit_cloth_frame`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the 160- and 139-byte cloth methods, the identity block at 0x2abc50 and the 1e-6 constant. No new corpus files.
- Out of scope: the buffers' contents.

## Phase

Phase 3, bodies and cloth. The cloth's start and frame swap.
