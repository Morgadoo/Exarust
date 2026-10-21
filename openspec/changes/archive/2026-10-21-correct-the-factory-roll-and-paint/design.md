# Design

Format pages: [`docs/formats/OTHER.md`](../../../docs/formats/OTHER.md),
Weapon factories, Weapon stats, Object database items and apparel
painting.

## Model

- The colour roll lives once in `exa_formats::factory::random_colour`
  (first mode and the mode-independent classes); the item palette's
  `part_colour` adds the modes that read the palette and calls it for the
  rest. The painted-steel class takes the palette's metal colour; a
  factory without a known palette uses the default 3.
- The paint keeps painting each part at its component's resolution.
  `paint::Tiling` carries the target texels per painted texel (the part's
  atlas region over the shade map) and the four tile offsets, so library
  textures repeat as the original's `MatTile` (divided by the material's
  first colour texture) and `DmgTile` (1024 texels) place them.
- A layer carries its tint and wear weight (`MatCol`), computed by the
  caller: the linear ramp and the material's weight for apparel, the
  lifted bytes and the finish level for factories.
- The normal map is painted for every layer before the specular pass,
  which measures creases on its stored two-channel bytes ±2 target texels
  around.
- Factory paints are cached by everything they depend on (stem,
  condition, layers, region, seed B), apart from the apparel cache.

## Open questions (answered)

- Is the stat pass's second curve the first one? REA on its instructions:
  it adds 1 to x, or divides 1 by 1 − x, without the amount.
- Which box do attach modes read? The component node's stored centre and
  half extents (the deforms change only the vertices).
- What scales the attach point? The parent part's recipe scale; a part
  without a parent keeps its position.
- Where do the material programs sample each texture? The decoded
  programs' tile uniforms, and the draws' uniform setup for their values.
- The bend's sine and cosine: the engine's packed polynomials with a
  rounded (not truncated) quadrant, pinned bit for bit.
