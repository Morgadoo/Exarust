# Proposal

## Why

The creation screen lays out a rail from the option count: the view shrinks by the unit times the count, and the thumb and start follow the count. Exarust has many creation behaviours but not the rail.

## What Changes

- The view is `width − before − unit × count`.
- Unless compact mode is on, the thumb is `(count − 4) × unit` and the start is `width − (count − 2) × unit`.
- Both rail values keep the low 16 bits.

## Capabilities

### New Capabilities

### Modified Capabilities

- `character-creation`: the rail layout.

## Impact

- `exa_game::create`, from `lay_out_creation_rail`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`: `SG_Character_Create` moves from 10% to 11%.
- Evidence: the 156-byte rail layout: the count, the unit global, the view width, the compact guard on the mode global against 12, and the two word stores. No new corpus files.
- Out of scope: the refresh call.

## Phase

Phase 5, character creation. The rail layout.
