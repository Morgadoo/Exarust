## Why

Textures that are not BC1, BC3 or BC5 (height maps in BC4, uncompressed
and packed images, odd sizes) were decoded on the CPU at their base level
only. They were uploaded without mip levels, so they shimmered and aliased
at a distance, unlike the original, which uploads the stored chain.

## What Changes

- Decode every stored mip level of an `.rfi`, not only the base level.
- Upload CPU-decoded textures with their full stored mip chain.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `texture-decoding`: all stored mip levels.

## Impact

- `exa_formats::rfi` and `exa_assets::texture`.
