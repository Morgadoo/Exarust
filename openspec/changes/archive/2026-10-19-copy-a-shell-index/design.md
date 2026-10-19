# Design

## Indices

`shell_indices` skips a section whose flags include 0x80000. A range that ends before it starts copies nothing. Otherwise each triangle from the first through the last, inclusive, contributes its three indices in order. Each index is replaced by the remap entry. A triangle past the list is a short error. An index with no remap entry is a remap error.

## Open questions

- The skinned section filter, and the stored size of 800, are not in.

## Docs

`docs/formats/OTHER.md` gains the index copy. `docs/PLAN.md` and `docs/COMPONENT-MAP.md` rate the shell mesh at 30%.
