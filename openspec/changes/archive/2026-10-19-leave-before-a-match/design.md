# Design

## Context

The career already hires combatants and one of each specialist. Before a match the roster is walked from index 0. A class whose high nibble is at most 2 is skipped. Exarust stores a profession, and a combatant is that class. The manager is not a roster row.

Health is an unsigned 16-bit word. Under 700 the death count rises by one and that row is removed. From 700 through 64999 the row is removed and the count stays. At 65000 or more the row stays. The walk returns after the first removal. A count of 0 returns before the loop.

## Goals / Non-Goals

**Goals:**

- `leave_before_match` hires through `Company::hire`, then reads health.
- A physician at 699 is removed and `deaths` becomes 1.
- A merchant at 700 is removed and `deaths` stays 0.
- A trainer at 65000 stays.
- A combatant at 0 stays, and the next specialist is the one considered.
- Two sick specialists: only the first is removed.
- An empty roster removes nobody.

**Non-Goals:**

- The kit release, the list compact beyond this roster, and the world-character unlink.
- The message log.
- Menus and the original save packages. The death count stays in memory. The Exarust session save does not store it.

## Decisions

1. The death count is `Company.deaths`, starting at 0. The session loader sets 0 and does not read a new field.
2. A departure is `Died` or `Left`, carrying the name, so a test can see which row went.
3. `XA_Arena` moves from 32% to 34%, the same share used for each prior career rule. The headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.

## Risks / Trade-offs

- Dropping the roster row does not release a kit. That callee is a separate function.
- A saved session does not restore deaths. The original package writer for that counter was not read.

## Migration Plan

- One method on `Company`. Hire, wages, and the day calendar stay as they are.

## Open Questions

- None for the two health lines and the single removal.
