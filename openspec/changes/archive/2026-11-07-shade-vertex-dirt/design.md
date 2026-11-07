# Design

Formats page: docs/RENDERING.md (Surfaces, Dirt), extending the material
record fields of docs/formats (dirt amount `0x30`, slot `0x8000000`).

## Colour stream

The model loader's group build copies the stored RGBA8 stream in the
group's vertex order as `Unorm8x4` on the colour attribute (the same
attribute and format the wound painter writes, so a wound on a dirty helmet
lowers the same alpha, as the original's painter writes channel `0x100`).
Assembly knows the group's material and drops the attribute again unless
the material shows a dirt mode: other surfaces upload nothing new, and no
new pipeline variant appears for them.

## Aspect

The original computes `DrtUVs` at every uniform fill from the bound diffuse
texture's size. The shader reads `textureDimensions` of the bound diffuse
map, so a streamed texture's placeholder and its final upload behave as
the original's texture object does.

## Open questions

- Mode 3 binds the map and sets `Obj.VrtCol` but no captured program reads
  it: answered from the shader library (no branch).
- Retail look of arenao1 with dirt: a capture is still to take (the
  `retail-exanima` skill); the formulas are the captured program's.
