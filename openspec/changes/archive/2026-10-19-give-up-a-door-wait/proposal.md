# Proposal

## Why

Phase 6 can hold a walker at a door. The wait that follows does not yet move the walker on when the door lets it through, or give up after ten seconds.

## What Changes

- A walker that is not waiting at a door stays as it is.
- A door that lets the walker through moves it past the door and lets the search run.
- At exactly ten seconds the wait still holds. One millisecond later the walker seeks again, clears the avoided node, and skips the cursor's low five bits.
- When both happen, seeking wins the phase and the search is already allowed.

## Capabilities

### New Capabilities

### Modified Capabilities

- `npc-steering`: the door wait on a path.

## Impact

- `exa_ai::path`, calling the door check already shipped.
- `docs/PLAN.md`, `docs/COMPONENT-MAP.md`.
- Evidence: the path step's door phase. No format field changes, so the corpus step is a skip.
- Out of scope: the heading sway, and the other path phases.
