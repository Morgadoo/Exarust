# Design

## Slot

`shell_slots` walks each vertex's half-open link range. A link names a triangle and a corner. The first mark other than 255 wins, and the walk stops. The stored section is that mark. The stored local index is the corner's index minus the section base, as a wrapping dword subtract. A vertex with none is absent. A short corner list or a missing base is an error.

## Open questions

- The stored size of 800, and writing the GPU buffers, are not in.

## Docs

`docs/formats/OTHER.md` gains the section slot. `docs/PLAN.md` and `docs/COMPONENT-MAP.md` rate the shell mesh at 50%.
