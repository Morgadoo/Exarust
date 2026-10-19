# Proposal

## Why

The line class has a small opening wrapper that no Exarust code models. It calls the parent's open and then loads the skin texture.

## What Changes

- Opening the line calls the parent once.
- It then calls the texture load once.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the line-class opening wrapper.

## Impact

- `exa_ui::skin`, from `open_line`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the wrapper's two calls. No new corpus files.
- Out of scope: the parent's open and the texture body.

## Phase

Phase 7, the interface. The wrapper, not its calls.
