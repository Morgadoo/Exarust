# Design

## Context

The approach and the door wait are in. Each of those phases eases a look toward the heading before it does anything else.

## Goals / Non-Goals

**Goals:**

- Leave any other mode alone.
- Copy the heading, start from the extracted angle, and ease toward the heading plus the offset.
- Draw a new offset only when the timer is behind the clock, then step the roll and set the next timer.

**Non-Goals:**

- The flag call that can run first.
- The angle extract. Its result is an input.
- The aim snap.

## Decisions

1. The timer refreshes when it is strictly behind the clock. Equal stays.
2. The roll step is `roll * 0x8088405 + 1` wrapping at 32 bits. The gap is the high half of that value times 4000. The next timer adds the clock and 4000.
3. The wrap is one step of two half-turns, the same comparison the attack yaw uses. Exactly half a turn does not wrap.
4. The ease is `(0.09 - posture * 0.06)` of the way from the extracted angle to the wrapped aim.

## Risks / Trade-offs

- The look is not yet called from a placed character. The function is the step's rule.

## Migration Plan

No data migration.

## Open Questions

- None for this slice. The look step was read before this change.
