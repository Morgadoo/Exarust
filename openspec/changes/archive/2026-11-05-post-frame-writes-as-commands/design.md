## Context

Follows `snapshot-the-bodies-and-cloths`. No `docs/formats/` page is involved.

## Decisions

- One queue for both modes instead of the stage-2 inbox (which only reaches the simulation's world): the queue's step is part of the fixed order, so it moves with the fixed step behind the bridge and stays in the frame in one world.
- A converted writer keeps its place in the frame and its decision; only the write moves. Component inserts and removals the frame owns stay in the frame, so the coverage check keeps seeing every type the simulation touches (one-shot systems are outside the moved schedules).
- Writes are deferred to the next step in one world too: the simulation reads the same values as before; a frame system later in the same frame reads the value before the write (a frame's delay for drawing).

## Risks / Trade-offs

- A one-shot system that names a type the bridge does not carry runs on the simulation's world without it; a failed run warns (`fixed command: …`). Converted writers use bridged types only.
- A held weapon's drawn transform differs during the first steps of the arena's load in some runs, in the frame and the simulation alike, with the body, grip and parents equal (`s4b2`–`s4b4`): most likely two unordered frame writers of that transform; to be checked with `--schedule-ambiguities`.

## Open questions

- None for reverse engineering.
