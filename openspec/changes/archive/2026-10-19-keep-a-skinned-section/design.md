# Design

## Filter

`shell_skinned_indices` copies a section only when its flags include bit 2. The hidden bit does not remove it. A range that ends before it starts copies nothing. Each triangle keeps its three indices in order, after the remap. A short list and a missing remap are the same errors as the object copy.

## Open questions

- The compacted vertex map, and the stored size of 800, are not in.

## Docs

`docs/formats/OTHER.md` gains the skinned filter. `docs/PLAN.md` and `docs/COMPONENT-MAP.md` rate the shell mesh at 35%.
