# Proposal

## Why

The daily scale takes a unit draw and a copied record. The unit comes from a 624-word generator, and a record copy walks plain fields, the gaps between them, and the tail. Neither is in the runtime yet.

## What Changes

- Seed 624 words, twist when the index is 624, and mix each word by the four shifts.
- Scale that word by 2 to the power -32.
- A mark that differs from the stamp seeds from the mark and reverses the mark.
- Copy a record by its plain fields: the gap, the field, then the tail. A short buffer or a bad field is an error.

## Capabilities

### New Capabilities

### Modified Capabilities

- `arena-career`: the unit draw the fee scale uses, and the plain-field record copy.

## Impact

- `exa_game::career::UnitGen` and `copy_record`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the 624-word twist, the four mix shifts, and the record walk. No new corpus files.
- Out of scope: managed strings, dynamic arrays, and the rest of the runtime copier.

## Phase

Phase 7, arena career. The unit and the plain-field copy, not the whole runtime.
