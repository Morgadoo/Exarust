# Design

## Map

`shell_marks` fills 255, then each section writes over its inclusive triangle range. A skinned section writes its index. Any other section writes 255. `shell_compact` walks each vertex's half-open link range. The first link whose mark is not 255 gives that vertex the next id, and the walk stops. A vertex with none is absent. `shell_apply_compact` replaces each index with that id. A short list or an absent id is an error.

## Open questions

- The stored size of 800, and writing the GPU buffers, are not in.

## Docs

`docs/formats/OTHER.md` gains the compacted map. `docs/PLAN.md` and `docs/COMPONENT-MAP.md` rate the shell mesh at 45%.
