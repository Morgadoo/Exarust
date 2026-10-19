# Proposal

## Why

A 48-byte method copies three measured globals and one unit-derived word into the skin's four measure fields. Exarust does not model it.

## What Changes

- The first, second and third measured words are copied to fields 8, 0xc and 0x10.
- Field 0x14 receives four units plus four.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the skin's measure fields.

## Impact

- `exa_ui::skin`, from `skin_measures`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the 48-byte method's four stores and the unit initialiser. No new corpus files.
- Out of scope: the code that changes the three globals.

## Phase

Phase 7, the interface. The measure copy.
