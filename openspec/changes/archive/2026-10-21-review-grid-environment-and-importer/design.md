# Design

Extends [docs/formats/RFC.md](../../../../docs/formats/RFC.md) (scene nodes,
props and their sub-chunks, placed objects, characters, materials, local
database), [docs/formats/TERRAIN.md](../../../../docs/formats/TERRAIN.md)
(palette versions, vegetation) and [docs/RENDERING.md](../../../../docs/RENDERING.md)
(rooms: door portals and light reach).

## Decisions

- **Plants untinted.** The vegetation loader's per-mesh helper fills the
  stream with smoothing key 2 (the normal stream) with (0, 1, 0); the draw's
  fourth attribute (four floats) is the tangent, rotated with the plant and
  its sign copied. No plant library has a colour stream (corpus), so the
  first pass's 0.75 grey tint is dropped and the picture of the plants
  returns to the previous one, apart from the shadow gate.
- **Density precision.** The sampler converts the byte to double, multiplies
  by the double 1/255, subtracts the double 0.2 and rounds to `f32`; the
  threshold 0.35 is a double compare and `2d + 0.1` adds in double. Exarust
  now evaluates the density the same way (38 of the bytes that grow plants
  changed in the last place).
- **Placement records.** The transform reader moves its cursor past the
  68-byte record and walks sub-chunks, recursing into transform and position
  records on the found node; the position reader never moves its cursor, so
  it applies no nested record. Node lookup goes through the node's child
  list with a 16-byte compare. The position list replaces the editable
  position elements only when its count is the position stream's; Exarust
  rebuilds the object's model from its file through `ModelLoader::edited_model`
  with the elements spread to the unified vertices by their position ids,
  normals recomputed by the smoothing keys, cached by a hash of the edits
  (at most 39 such records per level, so load time barely moves).
- **Palette fallback.** The opener tries `default.ftb` only in the branch
  where the named file is not found; the parse result of a found file is
  returned as is.
- **Light spill.** The callback computes the ¾ door point into one local and
  then compares the light with the other local, the light cell's centre; the
  first pass compared with the door point.
- **Portal sizes.** The quad's corners sit at the edge middle ± the table's
  first value along the edge, from −50 cm to its second value, so the table
  holds half widths and tops.

## Open questions

- What the position record's word sets on the node's soft-body copy (only 0
  ships). Answer with REA on the soft-body record's readers.
- Whether an instance node's GI points come from its owner (Exarust copies
  them). Answer with REA on the node linker and the GI point collector.
- The importer's "current locale" global: assumed to be the level's
  `locales.rdb` id. Answer with REA on its writers.
