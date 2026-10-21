# Proposal

## Why

The creation view runs a deferred callback when its pending flag and stamp allow, releases a tip when one is attached and stores the view with its callback. Exarust has the record, lists and refresh but not the tick.

## What Changes

- The deferred callback runs when the flag is set and the stamp plus 1 is below the clock; then the stamp takes the clock and the flag clears.
- The tick returns true either way.
- A tip is released only when present, and a view is stored only when present.

## Capabilities

### New Capabilities

### Modified Capabilities

- `character-creation`: the view tick.

## Impact

- `exa_game::create`, from `flush_creation_deferred`, `release_creation_tip` and `attach_creation_view`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`: `SG_Character_Create` moves from 18% to 19%.
- Evidence: the 81-byte tick: the pending byte, the stamp, the `+1` against the clock and the callback slot. The 39-byte tip release: the tip pointer and its release. The 59-byte view attach: the view and the callback registration. No new corpus files.
- Out of scope: the release and registration bodies.

## Phase

Phase 5, character creation. The view tick.
