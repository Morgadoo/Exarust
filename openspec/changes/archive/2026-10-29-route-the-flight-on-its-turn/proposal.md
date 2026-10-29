## Why

A fleeing NPC steered straight at its way out, through walls and pillars,
while the original's flee state plans its way out through the pathfinder and
follows the route. A first pass routed the flight but skipped the planning
turn and the follow test, and added an idle restart and a stand reset that
duplicated or echoed existing code.

Phase: docs/PLAN.md Phase 6 — AI (TMotileAI: flight, idle return).

## What Changes

- On a level with a navigation grid the flight picks and routes its way out
  only on the brain's planning turn (one planner a frame), plans again while
  no route is followed or within 150 cm of the goal, follows a route only
  when its ends lie more than 150 cm apart, and remembers the exit's room
  when a followed route ends in its room within 15 m of the threat.
- The idle return (the look after combat is left, the driver restarted at
  step 1) is kept as a tested function; the restart and stand reset stubs
  are removed (the stand reset is `motion::settle_look`).

## Evidence

REA/Ghidra on Exanima.exe: the flee state's tick, the path request (turn
gate, two points and a span over 150 cm, or forced), the stand reset and the
idle return, read from the instructions.

## Out of scope

The tick the original fights while no route is followed (the combat state
belongs to the combat work); the body's position copy the idle return and
stand reset make; calling the idle return from the chase.
