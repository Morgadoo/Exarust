# Design

## Context

The wake tick already refreshes a short weapon list when the clip base reaches 1. The full holding pass, the race rules, and the unarmed hand weight already exist as `equip`. This slice calls that function from the wake.

## Goals / Non-Goals

**Goals:**

- Hold each weapon record again when the clip base reaches 1.
- Weigh unarmed hands twice, or four times for a heavy-clawed race, and apply the ghoul and Fael results of that refresh.

**Non-Goals:**

- Steering along an accepted path.
- Replacing the short weapon list used when no record is held.

## Decisions

1. A wake that carries weapon records calls the shipped equip refresh and keeps its reach, thrust preference, and hand weight. A wake without records keeps the short list.
2. Hand weight is the mass multiplier, twice or four times the part. The ghoul's arm stiffness is 1. An unarmed hold at size 1 sets arm stiffness to one half, which a Fael keeps.

## Risks / Trade-offs

- Two refresh paths remain. The short list does not hold a record.

## Migration Plan

No data migration. No format page: no field changes.

## Open Questions

- None for this slice. The holding pass and the hand weight were already specified and tested.
