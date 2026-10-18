# Design

## Context

The duellist fights only after something else sets it engaged. Nothing rates a contact.

## Goals / Non-Goals

**Goals:**

- Rate one contact and let that rating start the duellist's guard.

**Non-Goals:**

- Hearing, roaming, and which swing to throw.

## Decisions

1. Stress is an input. The unidentified stress term is not invented.
2. The view weight is 1 ahead and 0 behind. The recorded product used minus signs, which invert that sentence, so the signs follow the sentence.
3. A duellist with no mind still waits to be engaged, so the arena fight is unchanged.

## Risks / Trade-offs

- One tick here is one update of the duellist, which is not yet locked to the 83 Hz body tick.

## Migration Plan

No data migration.

## Open Questions

The attack chosen inside the fight was not read.
