# Design

Extends `docs/RENDERING.md` (Terrain). No `docs/formats/` page changes.

**Bake.** `exa_render::terrain_composite` owns a bake queue and a
render-world pass. The queue is extracted every frame it changes; a bake
stays queued until the render world reports its ticket, so a bake whose
targets or textures are not yet on the GPU is retried. The render world
prepares at most one bake a frame and runs it once, by the scene view, as
four render passes into the set's targets (normal RG8, colour sRGB RGBA8,
gloss R8, displacement R8 at half the side). The original's groups of four
additive layers are folded into one stack per pass, as the per-pixel draw
already folds them, so the bake equals the per-pixel stack at each texel.

**Draw.** `exa_world::terrain::composite_draw` plans the steps from the
selector's holders, the host's targets and the finished tickets (a pure,
tested function), then swaps materials: a baked block takes a clone of its
tile's material with the set bound, its `TexPos` and the tile centre, and
the `COMPOSITE` flag. The shader reads the set as `shadetrn` does; the
set's v runs along +Z, so the parallax uses the lit program's own axes.
A rebuilt set (a new colour image) or a lost set restores the tile
material. A set copy is baked again instead of blitted.

**Room lights.** The rooms' light lists are built from the lights' current
positions each time a hidden block is ranked, with `RoomLights::add` and
`propagate`; the original adds a light once when it is created.

Open questions and how they were answered:
- How the draw reads a set: the shipped `shadetrn` programs (coordinate,
  parallax, normal and gloss channels); the terrain draw binds the set's
  four textures at units 0–3 per block.
- Whether a block without a set is drawn: the original's draw binds a set
  for every drawn block; Exarust keeps the per-pixel draw for those, since
  the selection reaches only 44.4 m and 64 sets.
- Visual parity: a GPU run of `arenao1` with a fixed scene seed, the
  composite draw against the per-pixel one.
