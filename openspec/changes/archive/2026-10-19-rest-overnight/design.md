# Design

## Context

Ending a day already pays wages, resolves a week, and drops an entry under 3276. The night raises health before that drop. Nothing raises it yet, so a pool of 3275 is dropped on day 7.

## Goals / Non-Goals

**Goals:**

- Add 6553 to each employee's health, or 9830 when a physician is hired.
- Stop the pool at 65535. A sum of 65534 is kept. A sum of 65535 or more becomes 65535.
- Drop an entry only when the rested health is still under 3276.

**Non-Goals:**

- The navigation countdown. Its writer was not found.
- The trainer's lessons, which run on the same night.
- The player's own record, which this company type does not store.
- The hire price and the reward item identities.

## Decisions

1. The step is chosen once for the night: 9830 when any employee is a physician, otherwise 6553. Every employee takes that step.
2. The sum is compared with 65535 before it is stored. A sum below 65535 is kept. Otherwise the pool is 65535.
3. The rest runs after the day counter moves and before wages and the drop, so day 7 sees the rested pool.

## Risks / Trade-offs

- A pool of 3275 can no longer be dropped on day 7, because one night raises it to 9828. The drop remains for a pool that is still under 3276 after the rest.

## Migration Plan

No data migration. No format page: no field changes.

## Open Questions

- None for this slice. The two steps, the cap, and the place of the rest in the night were read before this change.
