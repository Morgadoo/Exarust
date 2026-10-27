# Design

Rules: [`docs/formats/SAVES.md`](../../../docs/formats/SAVES.md), Level
transitions and the arena chunks; the crate READMEs.

## Decisions

- The outdoor preset takes the engine's shared generator separately from
  its local seed, because the rain setter draws thunder from the shared
  one; a zero seed takes a caller-supplied fresh seed (the original's
  clock seed).
- The follower placement returns the matrix position before each
  follower's own floor probe; the probe is the caller's, as for the
  player. Exarust stops at the six stored offsets where the original
  reads past its table.
- `scene_content` returns the lists in append order, since one node can
  join two.
- The arena tables hold a plain merchant record; the employees decide
  whether it is written.
- The settings chunk payload is built by the caller from the read tag,
  the old flag and the gate; the exporter only frames it.

## Not wired

Listed in the proposal's out-of-scope paragraph.
