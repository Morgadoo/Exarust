# Design

## Context

Four effects spend a fixed cost times a fixed factor. The power table has 33 rows, and the cast protocol changes a cost before that spend.

## Goals / Non-Goals

**Goals:**

- Store each recorded id and cost, and apply the sustained, repeat and spend multipliers.

**Non-Goals:**

- The tree layout, charge-up, effect meshes, and playing any power beyond the four already cast.

## Decisions

1. Sustained is id bit `0x10000`. Persistent is id bit `0x20000`. The domain is the `0xFF00` nibble: `0x100` mind, `0x200` force, `0x400` energy, `0x600` displacement.
2. A sustained cost is `cost * tick_ms / 1000`, then half with the sustain technique. An instant repeat is half only when the same power is cast again and the repeat technique is known.
3. Spend factors are 1.2 for blast and bolt, 2 for barrier, 0.3 for a mind power, 80 for vanish, and 1 otherwise. That keeps the long door at 1.

## Risks / Trade-offs

- A note also writes displacement factors as 0.8/80. Vanish is the 80. The long door stays at 1, which is what its cast already spends. The 0.8 was not tied to the long door.

## Migration Plan

No data migration.

## Open Questions

- Which mind powers, if any, do not use 0.3.
