## Context

Follows `bodies-as-snapshots`. No `docs/formats/` page is involved.

## Decisions

- The frame gathers the other types first, marks what they name (and its ancestors), then gathers the hierarchy of the marked entities: a body crossing for the first time brings its parents' frames in the same step.
- Entities the simulation sends are marked at the apply, already in sync; their unmarked ancestors are sent whole at the next gather.
- Descendants are not marked: a body's mesh and bone children stay home. A simulation system that needed one would show in the parity runs; none did.
- The cloth refresh runs only behind the bridge (in one world the particles are the simulation's own).

## Risks / Trade-offs

- A simulation system that reads the transform of an entity carrying no other bridged type would see it missing; `EXA_BRIDGE_WIDE=1` tells such a case apart.

## Open questions

- None for reverse engineering.
