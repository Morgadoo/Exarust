# Proposal

## Why

docs/PLAN.md, Phase 2 (static world), the scene-node unit paragraph: the
scene-node model handled every queued removal the same way at the frame
end, never gave a hidden node its owner back, kept the queue in order and
treated rooms as unrotated in the scene bounds and the active-sector test.
The scenenodes3 lane checked the frame, the queue writer and the
scene-bounds pass against the disassembly (REA listing of Exanima.exe).

## What Changes

- `exa_world::scene_nodes`: removals carry the original's three kinds
  (hide, delete, free). The frame drains the 256-entry ring into a pending
  list first, every drained node hanging from `_World`; the frame end gives
  a hidden node its owner back, waits on an owned deletion, gives a
  deletion its owner back before taking it out (or deferring it while it
  is held), frees a free entry without the wait, reports a deletion owned
  by `_World`, and replaces a finished entry by the last one.
- The light activation pass runs for every sector, as the frame calls it.
- Sectors carry a rotation: the scene bounds turn each room's boxes into
  world-axis extents by it, and the active-sector test meets the room's box
  along it.

Out of scope: putting the scene graph on the draw path, the node
destructor's own modes (another unit), the scene-wide lights' shadow
counter reset and the frame's callback slots.
