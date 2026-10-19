# Proposal

## Why

The cloth's destructor releases the object, frees four stored pointers and an optional origin, destroys three managed fields, frees a list of pointers and runs the base and final calls. Exarust does not model it.

## What Changes

- A positive free flag makes one release call and one final call.
- Each of the four stored pointers is freed when present.
- The origin at 0x120 is freed only when the cloth is its own root.
- Three managed fields are destroyed.
- A list frees each entry and then the list itself.

## Capabilities

### New Capabilities

### Modified Capabilities

- `cloth-step`: the cloth's destruction.

## Impact

- `exa_physics::cloth`, from `destroy_cloth`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the 416-byte destructor's pointer checks, three field destructors and list loop. No new corpus files.
- Out of scope: the release, field and base bodies.

## Phase

Phase 3, bodies and cloth. The cloth's destruction.
