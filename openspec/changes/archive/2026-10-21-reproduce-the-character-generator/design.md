# Design

Format pages: [`docs/formats/OTHER.md`](../../../docs/formats/OTHER.md)
(Object database items: the coverage pass) and
[`docs/RENDERING.md`](../../../docs/RENDERING.md) (the skin job, Hair).

## Decisions

- The slot table is read as the executable lays it out: each 0x48-byte
  record starts with the slot's own number, its parent and its flag word,
  so the record a pointer names begins 0x18 bytes before its child count.
  The earlier table had taken the next record's words as this slot's.
- The coverage pass and the assembler run per garment, as the builder
  does; the garment's own bytes are its occupancy words and the union is
  over the worn set's coverage words. The body mask tests the union's
  64-bit words against `0x80` and `0x40`, so only the innermost byte
  hides a body part; the running game's first-byte hiding already agrees,
  and the coverage-union hiding the lane had wired is withdrawn.
- The assembler keeps the original's stack order (the last child visited
  first), its swap-with-last removal and its first group; the stack grows
  instead of overflowing past 32 entries.
- Floating point follows the instructions: the hair material's byte units
  and feral lifts in double, the picker's widened steps, the hue sextants'
  reciprocal and double thirds, the frame normalise's `(x² + z²) + y²` with
  the processor's reciprocal square root estimate, the dot sums' `x + z`
  first, the point placement's `(x·r0 + y·r1) + (z·r2 + t)`.
- The body tree keeps the chunk kind, node flags, render flags and owner
  word apart; the shape builder's rest vectors are a caller closure.
- Not wired where the running game has no counterpart yet (merged mesh,
  rebuild, saved poses, off-hand items). The saved-pose system is removed
  rather than left writing world frames as local ones.

## Open questions and how they were answered

- Which material word the hair swap tests: a word the shipped hair
  material (`harhps1`, every `CharExt` style) holds at 1.0, so every hair
  part is tinted, as Exarust already did.
- The beard index has no upper bound in the original; stored values past 6
  give no beard here rather than reading past the tables.
