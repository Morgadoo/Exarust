# Design

## Context

The six named matches on the tier-0 board are fixed. The daily filler still draws which of seven templates occupies each slot. It replaces the roll from a clock before the first draw. This function starts from the roll it is given.

## Goals / Non-Goals

**Goals:**

- Gates, in order: earliest 0 limit 2 mask 0; the same again; earliest 1 limit 2 mask 0; earliest 2 limit 2 mask 0; the same again; earliest 0 limit 1 mask 3; earliest 2 limit 1 mask 0.
- Choose `(roll × 7)` high half after one step.
- Refuse when the limit is above 0 and the template has already been chosen that many times.
- Refuse when the slot index is below the earliest slot.
- Step again. The top three bits are `roll >> 29`. Refuse when those bits share a bit with the mask.
- Count an accepted template and move to the next slot. Six slots.

**Non-Goals:**

- The clock write that replaces the roll.
- Copying the template record and scaling its fee by the slot.

## Decisions

1. A mask of 0 never shares a bit, so that second step still advances the roll and the template is kept.
2. Mask 3 refuses a draw whose top three bits are anything but 0 or 4.
3. The fixed `standard_board` list stays. This function returns indices.

## Risks / Trade-offs

- `XA_Arena` moves from 28% to 30%. The game-modes section stays 0%. The headline stays 14.4% and 5.7%.

## Migration Plan

- One function. `end_day` does not yet call it, because that path also replaces the roll from a clock.

## Open Questions

- None for the six indices.
