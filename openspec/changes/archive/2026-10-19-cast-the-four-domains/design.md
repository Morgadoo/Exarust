# Design

## Context

The original dispatches a cast on the domain. Exarust has the focus bar and no powers.

## Goals / Non-Goals

**Goals:**

- One effect per domain that changes position, yellow health, a knock, or the mind, and that refuses when focus is short.

**Non-Goals:**

- Casting animations, particles, and the rest of each domain's list.

## Decisions

1. The long door and shock use factor 1. Blast uses 1.2, so it spends 0.12. Calm uses 0.3.
2. A full blast uses the knock cap of 30, then doubles it. The charge that fills that cap is not simulated.
3. Shock's strength is the arc's, passed in. The 150 ms gap is on the hit, not on the focus spend.

## Risks / Trade-offs

- Vanish spends eighty times its cost. That factor is not used for the long door.

## Migration Plan

No data migration.

## Open Questions

None.
