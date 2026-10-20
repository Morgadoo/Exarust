# Proposal

## Why

The item container's destructor releases the object, calls one of two per-entry release routines, destroys the entry array and runs the base and final calls. Exarust does not model it.

## What Changes

- A positive free flag makes one release call and one final call.
- Each counted entry gets one release call, through 0x107a90 when detached and 0x1079e0 otherwise.
- The entry array is destroyed once and the base runs once.

## Capabilities

### New Capabilities

### Modified Capabilities

- `inventory-pane`: the container's destruction.

## Impact

- `exa_items::pane`, from `destroy_container`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the 319-byte destructor's two entry loops and three teardown calls. No new corpus files.
- Out of scope: the release and base bodies.

## Phase

Phase 3, items and inventory. The container's destruction.
