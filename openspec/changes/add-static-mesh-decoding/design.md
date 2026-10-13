# Design

## Context

`docs/formats/RFC.md` describes the chunk container (verified) and a first
reading of the mesh chunk `0x3D03` (observed): a 16-byte name, flags, a
transform, bounds, typed vertex streams (`u16 semantic, u16 type, u32 count`),
then topology chunks `0x3DC0..0x3DC3`. Variants `0x3D01`, `0x3D06` and
`0x3D0C` exist (77, 22 and 13 instances). The community spec (Structs.txt)
mentions "count minus one" fields, separate position and texture-coordinate
streams with index remaps, per-material ranges, and a collision chunk
`0x3DD0B001`. It was written against the 32-bit build and may have drifted.

## Goals / Non-Goals

**Goals:**
- A total, Bevy-free decoder in `exa_formats::rfc` for static meshes.
- Exports that let a person check geometry and texturing in Blender.
- A corpus report that serves as the progress meter.

**Non-Goals:**
- Skinning, morphs, levels, full material parameters, collision interpretation.
- Bevy integration (Phase 2).

## Decisions

- **Evidence order.** Read the mesh reader in `Exanima.exe` through REA first
  (Ghidra provider: decompile and xref the routine that consumes `0x3D03`),
  then confirm every field with corpus statistics. Community notes only steer
  where to look.
- **Separate streams, unified vertices.** If positions and texture coordinates
  are indexed independently, build unified vertices keyed on the index pair at
  decode time, so the output is plain indexed triangles.
- **Triangulation.** Fan triangulation for convex polygons; ear clipping only
  if the corpus shows concave polygons.
- **glTF writer.** Write GLB by hand (a JSON chunk plus a binary chunk) to
  avoid a heavy dependency; textures are embedded as PNG produced by the
  existing image decoder.
- **Units.** Treat the stored units as centimetres unless the reader shows a
  scale factor; confirm against known object sizes (doors, tables, tiles of
  600 units).

## Open reverse-engineering questions

| question | how it is answered |
|---|---|
| Stream semantic and type codes (positions, UV sets, normals, colours) | REA: decompile the stream reader; corpus: histogram of `(semantic, type)` pairs |
| Polygon encoding in `0x3DC0..0x3DC3` (counts, index width, groups) | REA decompile; corpus: check that indices stay below stream counts |
| Meaning of variants `0x3D01/06/0C` | REA: the reader's dispatch on chunk tag; corpus: where they occur |
| Transform layout and handedness | REA; export and compare orientation in the game |
| Unit scale | known object sizes; level tile size of 600 units |

## Risks / Trade-offs

- Field layouts may have changed in the 64-bit build. Mitigation: the corpus
  run is the gate, not the documentation.
- Hand-written GLB may miss edge cases. Mitigation: validate output with the
  Khronos glTF validator during development (not a runtime dependency).
