# Proposal

## Why

The skin has a preparation method that guards the renderer setup with a global flag. Exarust does not model it.

## What Changes

- When the global is not 1, the method calls the setup twice (a mode 5 call and a second setup call) and returns 1.
- When the global is already 1, it makes no calls and still returns 1.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the skin renderer preparation.

## Impact

- `exa_ui::skin`, from `prepare_skin_renderer`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the guarded method's two calls and its return. No new corpus files.
- Out of scope: the setup bodies.

## Phase

Phase 7, the interface. The guard, not the setup.
