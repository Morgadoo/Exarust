# Design

## Context

Exarust's brain already ran the morale, contacts, target pass and the
look chain once a body tick (`brain::ai_tick`), and the flight walked a
fleeing brain through the NPC driver (`brain::flee_walk`). The roam and
patrol phases lived in `idle::tick`, unused.

## Decisions

- **The head stays in the AI tick.** The alert look and the attention
  need the turn, the gaze, the generator and the attended contact the tick
  already holds; the tick records whether the head took the tick
  (`Npc::head_took`) for the walk system.
- **The walk is its own system, after the flight.** The duellist clears
  every NPC's desired velocity each frame, so the walk runs after it, steers
  once a body tick and puts its velocity back between ticks, as the flight
  does.
- **Its own planning turn.** The walk keeps a `PlanTurn` beside the
  duellists' (the original has one); at most one idle route is searched a
  frame, which keeps frame time flat.
- **Units.** The phases and routes are centimetres; the ECS is metres and
  the conversion is at the system boundary.
- **Guards.** Arrival is measured flat (the body's root is not on the
  grid's floor) and a leg is given up after path length·20 + 2000 ms (the
  idle driver's deadline); both are labelled as Exarust's.

## Risks

Roaming NPCs now move in levels with a navigation grid; a stuck walker
gives up after its deadline and pauses. Frame time: one route search a
frame at most.
