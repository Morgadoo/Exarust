## Context

`brain::flee_walk` runs the flight in play. `route::plan` already carries
the original's path request (turn, the follow test) for the chase; the idle
walk keeps its own turn in `brain::idle_walk`. No docs/formats page applies
(behaviour only).

## Decisions

- Reuse `route::plan` rather than a flight-specific planner. The way out is
  picked inside the original's planner call, so Exarust picks it only on the
  turn too (the fled-room memory is touched no more often than the original).
- The flight keeps its own `PlanTurn` beside the idle walk's and the
  duellists' (labelled as Exarust's).
- The route is planned and followed from the feet, at their height, as the
  idle walk does; the goal's height 0 would pick the wrong grid layer.
- The idle walk's give-up deadline (length × 20 ms + 2 s, Exarust's guard)
  also guards the flight.
- While no route is followed the body steers straight to the exit, where the
  original runs its combat tick: labelled as a deviation.

## Open questions

- What the original does with the body position it copies on the idle
  return and stand reset: to be read from the readers of that field (REA
  xrefs).
