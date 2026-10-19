# Proposal

## Why

The splash screen has a 28-byte bounds method and a 157-byte destroy method. Exarust does not model either.

## What Changes

- Bounds writes the screen back to its first output, −1 to its second and returns −1.
- Destroy releases the screen when its free flag is set, clears the splash global, frees two stored pointers when present, releases a list when its count is positive, runs the base destructor and makes a final call when freeing.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the splash screen's bounds and destruction.

## Impact

- `exa_ui::menu`, from `splash_bounds` and `destroy_splash`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the 28- and 157-byte methods and their four conditional calls. No new corpus files.
- Out of scope: the release and list bodies.

## Phase

Phase 7, the interface. The splash lifecycle.
