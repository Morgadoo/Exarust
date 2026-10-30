# Proposal

## Why

Round 39's arena lane re-read the arena's weapon variant draw (the one a
generated opponent and a hire-list opponent armed from an event row use).
Exarust filtered the row's empty slots before indexing, so a row with an
interior zero could never draw that zero. The lane then misread the
slot test as signed; review against the instructions found it unsigned.

Phase: docs/PLAN.md Phase 7 — Game modes and content (the arena).

Evidence: the variant draw in the objdump listing and REA: three
`cmp dword, 0` / `jbe` tests (unsigned, so any non-zero word counts), one
LCG step, `movsxd` count times the state, `shr 32`, then the slot is
read at that index from the row as it stands, and the id is handed to the
item variant and condition roll.

## What Changes

- `exa_game::arena::pick_variant` counts the row's non-zero slots and
  indexes the row unfiltered; it returns the drawn id, 0 meaning no
  weapon. `pick_weapon_variant` keeps the drawn zero.
- `exa_game::arena_spawn::pick_weapon_slot` (wired through
  `arena_records`) indexes the row unfiltered the same way.

## Out of scope

- The item variant and condition roll after the draw (item system).
