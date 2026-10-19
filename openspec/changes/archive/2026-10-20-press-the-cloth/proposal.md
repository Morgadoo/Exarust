# Proposal

## Why

The cloth's pressure pass normalizes each normal, stores its offset, projects the edge's normal component, scales the correction by `0.1 / |local|²` above 0.2 and moves the normal by the corrected edge. Exarust does not model it.

## What Changes

- Gate 0x100 skips the whole pass.
- A normal shorter than 1e-6 becomes zero with offset −1.
- A longer normal is normalized; its offset is the normal dot the edge plus its length.
- The edge's normal component is removed; above a squared length of 0.2 it is scaled by the negative root of 0.1 over that length, else subtracted.
- The normal gains the corrected edge times the negative length.
- Afterwards the coupled updates run `n−1` times when `n` is at least 2, the normals are zeroed and the byte at 0x1fe is cleared.

## Capabilities

### New Capabilities

### Modified Capabilities

- `cloth-step`: the cloth's pressure pass.

## Impact

- `exa_physics::cloth`, from `apply_cloth_pressure`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the 809-byte method and its four vector helpers (`a−b`, `a−(a·b)b`, scaled add, subtract). No new corpus files.
- Out of scope: the coupled update body.

## Phase

Phase 3, bodies and cloth. The pressure pass.
