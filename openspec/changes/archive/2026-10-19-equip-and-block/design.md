# Design

## Context

Apparel occupancy is already decoded. The underwear rule runs at spawn. Hits always call `damage::resolve` and apply that outcome.

## Goals / Non-Goals

**Goals:**

- Equip and unequip by occupancy, including braies.
- Move an item between a container and a pack, then wear it.
- A parry or a shield block changes the outcome the hit path applies.

**Non-Goals:**

- The inventory GUI, parry clips, and weapons as bodies.

## Decisions

1. Overlap is any shared sub-part bit on the same layer byte of any bone.
2. Braies (item 8) are never returned to the pack. The groin test is the pelvis layer-0 bit already used at spawn.
3. `guard_outcome` runs on the resolved blow before health, blood and the knockback step. Open leaves it. A parry zeroes it. A shield replaces it with the leak: yellow 0.05·Q, Q = (1 − stamina)·(0.4 + 0.4·impact)·leak.

## Risks / Trade-offs

- The leak uses the blow's impact after armour. The original's shield path uses the contact's impact before the flesh armour roll. The test locks the formula as applied to the resolved impact.

## Migration Plan

No data migration.

## Open Questions

None. The constants are the ones already written down from the original.
