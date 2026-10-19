# Proposal

## Why

The cloth's activation sets four flag words, its phase, its 0.4 scale and its optional handle, origin and slow bits. Its transform method transforms all three particle buffers and the object's origin and extent. Exarust does not model either.

## What Changes

- Activation ORs 0x20000000 into 0x300, 0x10000 into 8, 0x204030 into 0xc and 0x6000 into 0x188.
- It sets phase 2 and scale 0.4, writes the handle when present, copies the root origin and ORs 0x8000 when the slow global is set.
- The transform makes three calls per particle, then the object's origin on flag 0x20 and its extent always.

## Capabilities

### New Capabilities

### Modified Capabilities

- `cloth-step`: the cloth's activation and transform.

## Impact

- `exa_physics::cloth`, from `activate_cloth` and `transform_cloth`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the 228- and 176-byte cloth methods and the 0.4 constant at 0x30dd48. No new corpus files.
- Out of scope: the transform and the base call.

## Phase

Phase 3, bodies and cloth. The cloth's activation and transform.
