# Design

## Context

The bracket clears losing records once the companies are already listed. Before that, a tournament picks 16 of 19 rivals. The bag starts as 0 through 18. Each stored id is one more than the chosen value, so the companies are numbered 1 through 19.

## Goals / Non-Goals

**Goals:**

- Draw 16 ids without replacement.
- Step a 32-bit roll by `× 0x08088405 + 1`.
- Choose `high(remaining × roll)`.
- Shrink the remaining count before the close check. Close when that new count minus 1 is still at least the chosen slot, by copying each later value down through that limit.

**Non-Goals:**

- Naming the companies or spawning their fighters.
- The six tournament events and their reward items.

## Decisions

1. The bag is a local array of 19. The function returns the 16 ids and the stepped roll.
2. The close copies from the chosen slot through `remaining - 1` after the shrink, which is the old count minus 2. The last slot needs no copy.

## Risks / Trade-offs

- `XA_Arena` moves from 26% to 28%. The game-modes section stays 0%. The headline stays 14.4% and 5.7%.

## Migration Plan

- One function beside the bracket clear. The company roster is unchanged.

## Open Questions

- None for the 16 draws.
