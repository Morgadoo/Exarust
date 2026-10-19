# Design

## Context

A roam can pick a wander point. Follow is the idle that stays with a leader instead.

## Goals / Non-Goals

**Goals:**

- Stand or run the activity when there is no leader.
- Close inside the facing-scaled personal stop, strictly.
- Use the 400 cm band, narrowed while following and widened while busy.
- Set urgency and replan outside that band.

**Non-Goals:**

- The path search the replan asks for. Whether it was accepted is an input.
- The activity tick and the obstacle nudge.
- Sleep.

## Decisions

1. The direction is the follower's place minus the leader's. The personal stop is (1 + facing · that direction / 2) × 100 cm. A zero distance is inside it.
2. Already following uses 0.7 of 400 cm, and that wins over the busy widening. Busy and not following uses 3 times 400 cm.
3. Urgency 2 is past 1600 cm or a sprint, urgency 0 is up to 800 cm, and 1600 cm itself is urgency 1. A sprint inside the band does not set urgency.
4. Already following and walking, the replan is strict past 100 cm from the planned spot. Not walking, a hidden leader stands instead of replanning. A new chase always replans, aims at the leader's aim point, stops 50 cm short, and on acceptance remembers the leader's place.

## Risks / Trade-offs

- The step is not yet called from a placed character.

## Migration Plan

No data migration.

## Open Questions

- None for this slice. The follow step was read before this change.
