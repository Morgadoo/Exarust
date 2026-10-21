# Design

Extends `docs/RENDERING.md` (Rooms and black fill; Light links). No
`docs/formats/` page changes: the model reads no game data.

## Representation

- The scene is a sector itself: the per-frame update reads the main views
  from the scene where it reads a sector's views, the scene bounds are
  written where a sector's bounds sit, and the batch insertion registers an
  ownerless node into the scene. Its first sector-list slot takes every
  view bit; the model makes that slot the scene's own sector. The global
  `_World` sector (the default parent outside scenes) carries the bit that
  makes the batch registration skip it; the model writes it as an owner of
  `None`.
- Frames are in sector space, as the original keeps them: a node directly
  under a sector takes its local frame. The sector's origin moves the main
  views into its space (active sectors) and places its box in the world;
  the union radius is measured about the world centre against
  sector-space corners, as the original does.
- The original's scene word and option word stay separate fields: bit
  0x10000 means "bounds split" in the option word and something else in the
  scene word.
- Lists append as the original does; the membership rebuild drops entries
  whose owner changed, keeping the meshes' order.

## Arithmetic taken from the instructions

- Box extents sum `|r₀|hx + |r₁|hy + |r₂|hz`; the box radius sums
  `(x² + z²) + y²`, the union radius `(x² + y²) + z²`; box centres are
  `low + (high − low) · 0.5`.
- The cell multiplies by the single `1/600` and floors (the engine's
  rounding helper is a floor); the cell byte wraps, then 0 becomes 1 and
  anything from `count − 1` up becomes `count − 2`, compared in 64 bits.
- Low and high spills are strict ordered comparisons; a NaN spills
  neither.
- The doorway reach tests pass a NaN; the facing test converts to double
  and passes a NaN.
- The fade's snap test is strict and a NaN dithers; 0.001 and 0.999 are
  doubles and a NaN takes the zero and partial sides.
- The scissor rounds with the processor's default conversion (ties to
  even) and projects one vertex per face, the clipped opening's corners.

## Open questions

- What the scene constructor puts in the first sector-list slot (the model
  assumes the scene itself): REA on the scene constructor.
- The draw path: how the scissor, the per-mesh masks and the dithered and
  partial draws reach the renderer: REA on the renderer's sector loop,
  then a capture of the running game at a doorway.
- Whether room sectors carry the grid size (the node passes read it from
  the sector): REA on the grid environment builder that creates the room
  sectors.
- The doorway records' stored opening: REA on the level importer's portal
  chunk and a corpus statistic over the grid levels (`cargo exa`).
