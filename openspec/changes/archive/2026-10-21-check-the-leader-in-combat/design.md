# Design

## Context

Before anything else in a tick, a following fighter that is not stepping
checks its leader; if it is already following with a route, the leader's
call ends the combat tick.

## Goals / Non-Goals

**Goals:** the check's arithmetic (double-precision eases, the 3D distance)
and its decisions.

**Non-Goals:** route planning, choosing the leader.

## Decisions

1. The check returns its decisions (stop, call, replan, gait); the caller
   owns the route and feeds `World::leader_calls` from `calls`.

## Open Questions

- None for the arithmetic; the route planner is another component.

## Docs

`crates/exa_ai/README.md`.
