# Design

Format page: [`docs/formats/RFC.md`](../../../docs/formats/RFC.md), Editable
meshes and Vertex colours. Rendering notes:
[`docs/RENDERING.md`](../../../docs/RENDERING.md).

## Channel rules kept

- A channel is keyed by its format word; element sizes are 12 (positions),
  8 (texture coordinates), 4 (colours) and 32 (skin).
- A channel the destination lacks is made and padded for its vertices with
  `0xff` (colours) or zero (everything else); index maps are offset by the
  destination channel's element count.
- A part without a map indexes element 0; colours and skin share the
  position map.
- Face flags default to 1 and edge bits to 7; the smoothing map is not
  joined and the face smoothing bits are copied.

## Departures

- Exarust joins unified render-form vertices, so a part without colours or
  weights gets white or zero vertices instead of leaving the destination's
  channel short.
- Smoothing keys are shifted so the two meshes' groups stay apart.
- The caller applies the node transform before the merge.
