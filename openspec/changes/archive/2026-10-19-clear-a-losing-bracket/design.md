# Design

## Context

The unwatched draw names one winning team and leaves the records alone. The bracket walk groups slots by a fixed width, resolves one group at a time, and writes 0 into each losing slot's id.

## Goals / Non-Goals

**Goals:**

- Skip an id of exactly 0 while scanning. A slot above 0 joins the team that contains it.
- Score `(count × 0.5 + 0.5) × strength sum`. The largest score starts at 0.
- On the first group, a forced id above 0 keeps that team and does not step the roll.
- Otherwise step the roll with a 32-bit `× 0x08088405 + 1`. The pick is the high half of the weight total times that word. Keep the first team whose running rounded weights are strictly above the pick.
- Write 0 into every other counted id. Leave the strength.
- Report whether more than one group was resolved.

**Non-Goals:**

- Spawning rival companies or chaining a live round.
- A short last group. The image then reads unset scores. The tests use full groups.
- A forced id that is not in the first group. The image then uses whatever index the caller left in a register.

## Decisions

1. A team stores the id of the slot that stopped the zero skip, then snaps back to the width boundary and sums every id above 0 in that window.
2. Rounding uses `round_ties_even`, the same convert the unwatched draw uses.
3. The pick multiplies the weight total, not one more than the total, and the compare is strict. That is the opposite edge from `draw_unwatched`.
4. A width below 1 resolves nothing. The image would divide by that width.

## Risks / Trade-offs

- `XA_Arena` moves from 24% to 26%. The game-modes section stays 0%. The headline stays 14.4% and 5.7%.

## Migration Plan

- One function beside `draw_unwatched`. Watched matches keep `finish`.

## Open Questions

- None for the full groups the tests use.
