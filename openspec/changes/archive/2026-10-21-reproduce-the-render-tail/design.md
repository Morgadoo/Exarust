# Design

Linked pages: `docs/formats/RFI.md`, `docs/RENDERING.md` (Light nodes,
Surfaces, Smoke and mist, Pixel-space 2-D batcher, Terrain),
`docs/formats/FLB.md` (material types and factory list).

## Decisions

- **Image sizes.** One function per original routine: a level's size from
  the word's block, bit-packed and byte-aligned fields; a face's size adds
  the halved levels only under the chain flag; the converters' pixel count
  adds them the same way and multiplies by six for a cube. The cube test is
  the header flags under `0x70000000` equal to `0x40000000`; the format
  word has no cube branch.
- **Conversion.** The registered formats are the existing pixel table (14
  eight-bit and 6 float formats, each with its fallback); a conversion
  takes the first format whose id is the image's word and runs its
  converter over every face and level. The original's block-compressed
  registrations have no Exarust converter, so such images do not convert.
- **Light flicker.** Steps, bounds and the scale update are computed in
  double precision as the instructions do; NaN branches follow the
  unordered compares. The frame time is whole milliseconds held to
  0…16.667. The engine's generator keeps a stream of its own (the original
  shares one across the engine). The emissive share is written into the
  parent mesh's first part; the draw-side reader of that value is not
  traced, so the part's emissive colour is scaled, on a material copy so
  shared materials stay untouched. A light at rest never touches its mesh.
- **Removals.** Code whose behaviour did not match the assembly and had no
  caller was removed rather than corrected where a correct port needs
  machinery Exarust does not have (the icon renderer's still capture, the
  kd-tree thread, the node hand-off and deferred queue).

## Open questions

- What the sub-object emissive scale multiplies in the draw (REA on the
  object uniform writer).
- Whether any shipped light body carries the flicker words (corpus scan of
  every level's light bodies with `cargo exa`).
- The registration order of the block-compressed formats relative to the
  pixel table (REA on the start-up order); it matters only for duplicate
  ids.
