# Design

## Context

Each AI tick one brain runs its target pass: it classifies every seen and
every nearby character against its contact list, then flees, fights or
calms. The flee state runs while the level is 4: it plans a route out of
the room through a goal callback that picks the exit most opposite the
threat.

## Goals / Non-Goals

**Goals:** the classifier, the list's admission, the pass's choice and
morale updates, the way out and the flee tick as pure functions.

**Non-Goals:** the perception, the path planner, the room graph, greetings
and encounters (other parts), the running game's flight (no rooms there).

## Decisions

1. **One classifier.** `sense::decide` builds a contact record from its
   rating and runs `contacts::classify`, so the duellist's engagement and
   the sleeper's waking follow the original's order.
2. **Rooms as data.** The way out takes the room's exits (point, room
   number, shut door) and the fled-room memory; the caller reads them from
   its room graph.
3. **Dead store.** The classifier's clearing of a contact's threat term
   past an alarm of 10 writes a working copy after it is stored, so it has
   no effect; Exarust leaves the stored record alone.

## Open Questions

- What the contact's sighting weight and threat term are (the perception
  fills them): read when the perception is ported.
- The stress base the classifier adds: who writes it.

## Docs

`crates/exa_ai/README.md`; research notes in `context/re/npc-combat-state.md`.
