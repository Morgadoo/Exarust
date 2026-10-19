# Design

## Room

`shell_index_room` sums `end - start + 1` for each section whose flags include bit 2. The subtract wraps. A section without that bit adds nothing. The byte count is 12 plus 8 plus 4 times the triangle count. The kind is 3, the base is 0, the end is the count minus 1, and the vertex end is the kept count minus 1.

## Open questions

- The stored size of 800, and uploading the GPU buffers, are not in. The count itself is the room taken when the upload flag is clear.

## Docs

`docs/formats/OTHER.md` gains the index room. `docs/PLAN.md` and `docs/COMPONENT-MAP.md` rate the shell mesh at 55%.
