## Why

With shadows on, torches threw large angular shadows across the menu's
walls and desk. Alpha-cut materials such as cobwebs were drawn into the
shadow maps as solid sheets, because the material had no shadow-pass
shader. The original cuts them in its shadow pass, and faint or flagged
surfaces cast no shadows at all.

## What Changes

- A depth-pass fragment shader for `ExaMaterial`. It applies the material's
  alpha cut, with the same rule as the colour pass.
- Surfaces with opacity ≤ 0.8, or with material extra flag `0x4`, cast no
  shadows.
- The sun's shadow map is 4096², as at the original's top shadow detail.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `rendering`: shadow casters.

## Impact

- `exa_render` (`exa_prepass.wesl`, `material.rs`, `lib.rs`),
  `exa_assets::model` and `docs/RENDERING.md`.
