# Proposal

## Why

The Kaiser chain is modelled but not used by the texture loader. The original generates mips for textures stored as a single level (icons, cursors, terrain, skin) and uploads them. Exarust decodes only the stored levels, so such textures shimmer.

## What Changes

- An RGBA8 image with one stored level, power-of-two dimensions and both sides at least 4 gets the Kaiser chain appended.
- Its mip level count becomes the base plus the generated levels.
- Every other image keeps its stored levels.

## Capabilities

### New Capabilities

### Modified Capabilities

- `rendering`: the Kaiser mips in texture loading.

## Impact

- `exa_render::mipmap`, from `build_mip_chain_rgba8`; `exa_assets::texture` uses it when `needs_kaiser_mips`.
- `docs/PLAN.md` and the generated `docs/COMPONENT-MAP.md`: `RFP_Mipmaps_Kaiser` becomes **40%** in the source rating. The headline rises slightly.
- Evidence: the level probe and the generation entry, plus the decoded texture path in `rfi_to_image`. No new corpus files.
- Out of scope: the NPOT fallback, the format converters and the GL upload.

## Phase

Phase 2, rendering. The Kaiser mips in texture loading.
