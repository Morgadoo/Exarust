# Design

## Context

The original bakes each 6 m block of terrain into composite textures from
every material the tile uses, four materials per pass. Exarust evaluates the
stack per fragment. Its shader took four materials, one per vertex-colour
channel, so tiles with more lost materials and showed whatever lay under
them, the black flat-tint fill among others.

Shipped terrains use up to 12 materials per tile, 11 per 6 m block and 10
in a single quad. Splitting tiles into four-material regions cannot cover
the quads with more.

## Decisions

- **16 layers, separate bindings.** Palette textures differ in size (512 to
  2048) and are BC-compressed, so they cannot share a texture array without
  decoding them, which would cost about 1 GB of video memory. Each layer
  keeps its loaded images; the material binds 16 diffuse, 16 surface and 16
  normal textures, read through one repeating sampler. The device limits
  Bevy requests are the adapter's, far above 48 textures per stage.
- **Splat texture.** Over opacities for 16 layers do not fit the vertex
  colour. A 241 × 241 texture of four RGBA8 layers per tile holds them, one
  texel per vertex, sampled bilinearly at the texel centres: the opacity
  between vertices is bilinear instead of linear over each triangle.
- **Uncovered layers skipped.** A layer whose opacity is zero at a fragment
  is not read, so a fragment pays for the layers that reach it.

## Risks

- RGBA8 opacities quantise to 1/255. The CPU passes (height, vegetation)
  keep the exact opacities.
