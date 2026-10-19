# Design

## Context

`finish` credits tier-0 experience to the first combatant. The award routine scales that pool by the tier and splits it. Seven more match templates sit beside the six standard ones.

## Goals / Non-Goals

**Goals:**

- Scale the pool by 1 + tier/4 and split it by contribution counts.
- Record the seven special templates at the values stored for tier 0.

**Non-Goals:**

- The hire price, fee and prize scaling, wages, the day calendar, and tournament generation.

## Decisions

1. Tier 0 with an empty share list stays the existing credit: the whole pool goes to the first combatant, so the duel scenario is unchanged.
2. Shares line up with combatants in roster order. A short list is padded with zeros. Extra shares are ignored.
3. The integer remainder goes to the earliest largest remainder, so the parts sum to the pool.
4. Special fees and prizes are the stored tier-0 numbers. A higher tier does not change them here.

## Risks / Trade-offs

- The original splits by a per-fighter contribution counter that this career does not yet count during a fight. The caller passes the counts.

## Migration Plan

No data migration.

## Open Questions

- How a tier scales a fee and a prize was not recovered.
