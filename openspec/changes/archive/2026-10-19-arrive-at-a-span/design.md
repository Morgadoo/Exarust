# Design

## Context

A door wait already gives up after ten seconds. The approach phase is what starts that wait, from the midpoint of a waypoint span.

## Goals / Non-Goals

**Goals:**

- Leave any other phase alone, and leave a busy walker alone while the clock is still at the stamp.
- Treat a missing span, and a horizontal distance past 250 cm, as a miss.
- Set only the high skipped bit on the first miss. On a later miss, also skip the cursor's low five bits.
- Start the door wait when the distance is 250 cm or less.

**Non-Goals:**

- The aim snap that runs after the choice.
- The other path phases.

## Decisions

1. The midpoint is the first end plus half the way to the second. The reach is the squared horizontal distance, 62500, so exactly 250 cm is near.
2. The busy hold is `now <= stamp` and the busy flag. Either one failing falls through to the distance.
3. The high bit stays set. A later miss ORs the cursor bit, `1 << (cursor as u32 & 31)`.

## Risks / Trade-offs

- The walker is not yet called from a placed character. The function is the phase's rule.

## Migration Plan

No data migration.

## Open Questions

- None for this slice. The approach was read before this change.
