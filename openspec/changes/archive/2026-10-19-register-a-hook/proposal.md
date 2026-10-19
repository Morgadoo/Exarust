# Proposal

## Why

A cleared mark does not register its hooks, and a shown value does not record that the indirect call activates an action set.

## What Changes

- A cleared mark registers code 0x8001 unless the hold byte is set. A code with a clear high nibble gains bit 0x1000.
- A non-zero world byte registers the wide hook twice, then drops the plain and lab hooks.
- Raising the mark drops those hooks, drops the play hook, and inserts the watch hook at key 4.
- A shown value activates the action set with the two platform words. The Steam library is not loaded.

## Capabilities

### New Capabilities

### Modified Capabilities

- `arena-career`: hook registration and the action-set call.

## Impact

- `exa_game::menu`, from `MatchGate::begin_match` and `MatchGate::set_session_aside`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the hook-list stores and the resolved action-set call. No new corpus files.
- Out of scope: loading the Steam library, and the bodies of the registered hooks.

## Phase

Phase 7, arena career. The lists and the call, not the Steam library.
