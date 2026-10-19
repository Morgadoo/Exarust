# Proposal

## Why

The combatant's reset zeroes its count, four qwords, two words and a byte, sets a sentinel and clears the body's hit and contact pointers; its measure method stores a reach and a scaled body value. Exarust does not model either.

## What Changes

- The reset zeroes the listed fields, sets 0x7a0 to −1, clears the body's 0xe38 and 0xe50 pointers, clears a marker's two words and decrements a reference's count before clearing it.
- The measure pair stores the reach at 0x7b0 and the body value times 1.0 at 0x7b8.

## Capabilities

### New Capabilities

### Modified Capabilities

- `combat`: the combatant reset and measures.

## Impact

- `exa_combat::damage`, from `reset_combatant` and `refresh_combat_measures`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the 216- and 62-byte methods and their stores. No new corpus files.
- Out of scope: the base call and the reach function.

## Phase

Phase 3, bodies and combat. The reset and measures.
