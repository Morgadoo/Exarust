# Design

## Context

The salvage lane ported the sampler pipeline, with the Run overlay as its
second step, and the movement decision's run blend and stop distance. The
overlay's weight came from main's `weight-the-sprint-overlay`, which used
the clip clock's sprint gates for it. The original's sampler tests only
the request's sprint latch bit before slerping toward Run: the run blend
with it, 0.7 × the blend (a double product) without. The original's
locomotion routine picks the sprint row (for the clip rate, the thrust,
the push and the stop distance alike) only with the latch, a stance level
above 0.9 compared in double precision and a speed above 3.5 cm a tick.
See [formats/RSQ.md](../../../docs/formats/RSQ.md) (Movement decision,
Motion playback).

## Goals / Non-Goals

**Goals:**

- The overlay reads the latch alone.
- The stop distance's sprint row reads all three gates.

**Non-Goals:**

- Moving the capsule by the sprint row's thrust beyond what it does now.
- The run blend's bleed terms.

## Decisions

- `Pace` carries the latch beside `sprinting`; the clock keeps reading
  `sprinting`, the overlay reads `latch`.
- `locomote` takes the stance level the movement stage reads from the
  body's balance (0 without a drive: no sprint row).

## Risks / Trade-offs

- With Shift and a far cursor the pose now meets the run bone as soon as
  the blend is full, also while the body is still slow. That is the
  original's.

## Open Questions

None: both rules are read from the binary.
