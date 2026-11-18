## Why

Lane `itemtex` (roadmap B4, port queue Q16/Q05). The rulings review refused
fifteen painter rows (the TProcMaterial family setups) because the CPU
painter read every library texture at its nearest texel while each load asks
for linear filtering (word `0x70133`); the same review left DEC-12 open on
two programs with no CPU counterpart (the gemstone pattern colour program and
the alternate steel normal).

## Evidence

- REA on the loads: every library texture of the 24 material setups loads
  through the texture loader with `0x70133` (linear magnification and
  minification, no mip chain, repeating, anisotropy mode 3). The factory
  region builder loads each component's `d`, `n` and `m1`..`m4` maps with
  `0x133`; the apparel layer-list builder loads its block's `d`, `n` and
  masks with `0x133` too. The emblem pictures load with `0x70133`.
- `textures.flb` (`exa-tool shader --shader-library textures.flb --dump`):
  every program reads at `vrTex`, the fragment's centre, times its tile
  uniform.
- `exa-tool corpus paint-maps` (new): the 62 library textures, 3152 factory
  component maps and 2136 apparel maps all upload level 0 at texture detail 3
  (detail classes 0 and 2 only); no component map and two apparel maps set
  their own wrap bits.
- The pattern pass is called only by the factory job's draw, for a region
  whose picture slot is set; the factory's region builder (asm read) never
  writes that slot and the apparel builder zeroes its blocks. The alternate
  normal program is chosen only for region flag 8 (part flag 4), whose one
  shipped part (`shield test`) no item names.

## What changes

- Library textures and emblem pictures: the bilinear mix of the four level-0
  texels around the fragment-centre sample point, repeating.
- A job's own maps (shade, normal, masks, and the shade scratch's inputs):
  the bilinear mix at level 0 with clamped edges, magnified or minified, for
  apparel and factory jobs alike.
- `exa-tool corpus paint-maps` tallies the painter's source maps.
