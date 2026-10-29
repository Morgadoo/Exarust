# Design

Links: `docs/PLAYER.md` (The item cursor, Combat mode); no docs/formats page
changes (the marker is a node name rule, not a file field).

## Decisions

- Markers are a component on model nodes (`exa_world::markers`), set where
  the node is spawned; the item cursor gathers them only while a lift
  holds something, so the walk costs nothing otherwise.
- The template is the record payload's parent word, kept on the placed
  root; the held body walks up to the nearest placed object for its ids.
- A marker's frame is copied whole into the held part (other parts keep
  their offsets); the grab is then held still.
- The approach bend reads a vector whose x the original overwrites and
  whose second part is an unwritten stack word; Exarust takes zero there
  (labelled a guess).

## Open questions

- The space-free test at a marker (spheres against other bodies): needs a
  body overlap query; answered later with REA on the overlap routine.
- Which body target the goal reads: Exarust uses the behaviour's
  auto-target; a capture of a locked fight would confirm.
