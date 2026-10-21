# Design

## Context

The event spawn walks the bracket's entry slots in runs of the team size
and places every member through one ring routine: `3 − a`, times a double
just under a sixth, is a fraction of a turn. Rule page:
[`docs/formats/OTHER.md`](../../../docs/formats/OTHER.md) (Arena tables).

## Goals / Non-Goals

**Goals:** the grouping, the radius, the spread and the place, exactly.

**Non-Goals:** spawning, AI teams, re-dressing.

## Decisions

1. Slots are the entrants' ids; a zero team size forms no team (the
   original would divide by zero).
2. A company opponent's level is left to its generator (`level: -1`).

## Open questions

- `exa_world::start::arena_ring` approximates this ring with an exact sixth
  and the standard library's trigonometry; replacing it needs the engine's
  sine in `exa_world` (a later change, checked against a retail capture of
  a team event).
