# Design

## Context

Arena mode is a manager, a roster and a match board. None of that runs in Exarust.

## Goals / Non-Goals

**Goals:**

- Create a company, hire within the caps, and resolve a duel.

**Non-Goals:**

- The hire price, the shop, wages, seasons and the board UI.

## Decisions

1. Money is a copper total. Gold and silver are that total split by 12.
2. Seven combatants fit, because the manager is the eighth fighter.
3. Experience uses tier 0, so the tier factor is 1. It is credited to the first combatant.

## Risks / Trade-offs

- The fee and prize tables are the unscaled template values. Tier scaling was not dumped.

## Migration Plan

No data migration.

## Open Questions

The hire price is unread, so hiring does not spend copper.
