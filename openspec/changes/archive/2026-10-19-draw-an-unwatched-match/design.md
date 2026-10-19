# Design

## Context

The career already pays a watched match from a win flag. An unwatched match walks the fighters, groups them by team, and draws a winner. A fighter whose mark has bit `0x800` is left out. Mode 16 also leaves out the player's team.

## Goals / Non-Goals

**Goals:**

- Group fighters in first-seen order and score `(count × 0.5 + 0.5) × strength sum`.
- Keep the largest score, starting from 0, and replace it only when the new score is greater.
- Weight each team by the square of score over that largest, times 100, converted with round-to-nearest-even.
- Step the roll with a 32-bit `× 0x08088405 + 1`, then take the high half of that word times one more than the weight total.
- Pick the first team whose running rounded weights are not below that pick. If none qualify, the first team stands.

**Non-Goals:**

- The other bracket walk, which clears losing records and can force the first group's winner.
- A physician saving a knocked-out employee.
- The injured drop on a multiple of 7. That already runs in `end_day`.

## Decisions

1. The public fighter carries `aside: bool` rather than a mark word. The draw leaves that fighter out, which is what bit `0x800` does.
2. Mode 16 is `FORFEIT_DRAW`. Any other mode counts the player's team.
3. Rounding uses `round_ties_even`, matching the convert-to-integer used when the rounding mode is nearest-even.
4. No counted fighter still steps the roll and returns no team. The retail path then reads an unset slot. This runtime does not.

## Risks / Trade-offs

- A largest score of 0 is not a tested match. Positive strengths are the cases the spec names.
- `XA_Arena` moves from 22% to 24%. The game-modes section stays 0%. The headline stays 14.4% and 5.7%.

## Migration Plan

- One function beside the career. Watched matches keep `finish` and `finish_shared`.

## Open Questions

- The bracket that clears losing records is a separate walk. It is not this draw.
