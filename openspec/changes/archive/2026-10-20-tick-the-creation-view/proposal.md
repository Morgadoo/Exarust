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
- Evidence: the 81-byte `0x10012c7f0`: the pending byte `0x1e4`, the stamp `0x1e0`, the `+1` against the clock `0x1004997b0` and the callback slot `0x1b8`. The 39-byte `0x10012c5c0`: the tip at `0x1d8` and its release `0x10006e890`. The 59-byte `0x10012c7b0`: the view at `0x1e8` and the callback registration at `0x178`. No new corpus files.
- Out of scope: the release and registration bodies.

## Phase

Phase 5, character creation. The view tick.
