# Design

## Context

Ending a day already rests health and pays wages. A trainer on the roster teaches after the rest and before the wages. Nothing teaches yet.

## Goals / Non-Goals

**Goals:**

- Teach each combatant, and not a merchant, a physician, or the trainer.
- Use the first skill under level 5 whose technique at that level is non-zero.
- Add the lesson only when that technique sits in a trainer slot below the trainer's level.
- Divide 1000 by one plus the sum of the six levels, and snap a word that crosses a multiple of 5000.

**Non-Goals:**

- The player's own record. This company type does not store it.
- Copying a loadout when the level changes.
- Item prices and emblems.

## Decisions

1. A level is the skill word divided by 5000. The wage rank stays the separate level array already shipped.
2. The trainer's known slots are the ones strictly below the trainer's level, and at most the six stored slots.
3. The grant is a 16-bit add. A level that increases replaces the word with the new level times 5000.

## Risks / Trade-offs

- A lesson that does not cross 5000 leaves the wage rank unchanged, because wages read the level array and the lesson writes the raw word.

## Migration Plan

No data migration. No format page: no field changes. A saved roster loads with empty training words.

## Open Questions

- None for this slice. The skill pick, the match, the 1000, the rank, and the snap were read before this change.
