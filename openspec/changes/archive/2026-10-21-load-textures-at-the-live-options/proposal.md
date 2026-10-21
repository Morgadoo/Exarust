# Proposal

## Why

The original's texture quality and texture detail settings write the
globals its texture loader reads: the filter mask, the anisotropy level, the
LOD bias base, the first uploaded level's detail and, through the
super-sampling level, the LOD bias. Exarust's texture loader always used the
retail user's values (quality 5, detail 3, 4× super-sampling), whatever the
settings file or the Settings page said.

## What Changes

- Whenever the live options change, the texture loader takes the live
  texture quality (its mask, anisotropy level and LOD bias base), texture
  detail and super-sampling level, keeping its driver limits. Textures
  decoded afterwards follow them; textures already decoded keep theirs.

## Capabilities

### New Capabilities

### Modified Capabilities

- `rendering`: texture sampling follows the live options.

## Impact

- `exa_game::hw_config` (`texture_settings`, the options system writes the
  model cache's texture settings).
- Docs: `docs/UI.md` (Settings options). `docs/PLAN.md` is the integrator's.
- Phase 6 (the hardware configuration), the texture loader of phase 2.
- Evidence: the settings' handlers and the texture loader's globals as
  `exa_assets::texture::TextureSettings` already documents them (REA).
- Out of scope: reloading textures already decoded when a texture slider
  moves (whether the original does is not traced).
