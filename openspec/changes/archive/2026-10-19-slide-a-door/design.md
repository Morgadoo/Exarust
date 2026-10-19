# Design

## Context

The hinge already creaks, counts a pull, and runs a motor. A sliding door is the other travel: a fraction along its link, with its own creak and its own open and close commands.

## Goals / Non-Goals

**Goals:**

- Creak from the stored energy.
- Step a motorised open and a close, hold a shut door, and unlatch it on the seventh pull.
- Jump a plain open to 0.6, park a frozen door, and keep a held hand point inside 60 cm.

**Non-Goals:**

- The partner door, the lock, the slam's random pitch, and the link law that pulls the body toward the fraction.

## Decisions

1. The creak stops at energy 0.015. Otherwise the volume is `0.2 · sqrt(energy)` and the pitch is `min(0.2 · energy + 0.7, 1)`.
2. A motorised open subtracts 0.01 and uses smoothing 0.1. Below 0.01 the fraction is 0 and the command ends. A close adds 0.075, uses smoothing 0.05, and at a fraction past 0.99 sets it to 1, latches, and ends on the shut mode.
3. A countdown below 2 ends the command and that tick still applies the crawl or the close.
4. Shut and not held forces fraction 1 and smoothing 0.5. Held below 0.995, the count rises by 1 up to 20 and then falls by 4. Below 0 the door leaves the shut state.
5. A plain open writes fraction 0.6 and 80 ticks. A motorised open writes 300 ticks and does not jump. A frozen door copies its parked fraction and is passable. A hand offset whose length is past 60 cm is scaled back to 60. A non-finite offset is left as it is.

## Risks / Trade-offs

- The placed door does not yet call this each tick. The function is the travel's rule. The bottom knock and the slam report a volume and leave the random pitch out.

## Migration Plan

No data migration.

## Open Questions

- None for this slice. The travel constants were read from the sliding door step before this change.
