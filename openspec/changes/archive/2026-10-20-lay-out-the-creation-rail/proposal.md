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
- Evidence: the 156-byte `0x10012e980`: the count at `0x1f8`, the unit `0x1002ad3f0`, the view at `0x160 + 0x28`, the compact guard at `0x4effa0` against 12, and the word stores at `0x98 + 0x140` and `0x98 + 0x13c`. No new corpus files.
- Out of scope: the refresh call.

## Phase

Phase 5, character creation. The rail layout.
