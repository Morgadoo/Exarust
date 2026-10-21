# Design

Formats: [`docs/formats/RSQ.md`](../../../docs/formats/RSQ.md) (Skinning),
[`docs/formats/OTHER.md`](../../../docs/formats/OTHER.md) (appearance block),
[`docs/RENDERING.md`](../../../docs/RENDERING.md) (Skin).

## Decisions

- **Shared marks.** The builder keeps one mark byte per position (in a
  group, widened a box) for the whole mesh, cleared once per node. Exarust
  keeps the same vector in its builder instead of a set per group, so a
  position belongs to the first group that reaches it and the later-group
  overwrite only matters where the original's does.
- **Falloff and share are different classes.** The share reads the high
  byte (0xC0, 0x30, 0x0C); the edge falloff reads byte 2 with the same
  patterns. They are two lookups.
- **Exact float order.** Distances, dot products, lerps and box updates use
  the operand order of the instructions: `MINPS`/`MAXPS` semantics for the
  trace's box, ordered compares for the busy box, `clamp` for the edge
  parameter (it keeps NaN as the two compares do), a double compare for the
  0.999 reach.
- **The tone in software.** The tone pipeline runs on the CPU once per tone
  with the engine's fast logarithm and exponential re-expressed from their
  constants; it is checked against an independent single-precision
  emulation.
- **Watertight check.** A real-data test pairs every two vertices of
  different parts at one rest position and requires equal weights on the
  shipped human and ogre bodies.

## Open questions

- Which in-memory node flags choose the joints and the apparel mode: the
  flag words of the file do not map onto them in the pack; answered later
  by REA on the node reader.
- The group stack the apparel matching consults for nested garments: its
  reader is outside the pack; answered by REA on the recursive driver's
  callers.
- The race flag that picks the men's skin set for some races: the pack shows
  the bit test, the race record field it reads is not confirmed; answered by
  REA on the race reader against `races.rdb`.
