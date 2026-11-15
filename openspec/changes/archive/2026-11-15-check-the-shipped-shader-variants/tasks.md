## 1. Variant listing

- [x] 1.1 `exa_assets::model::record_path` (the material build's start) and `exa-tool shader --variants`
- [x] 1.2 Run on 0.9.5's data: 147 groups, 7533 distinct programs; compare each group's template flags with `exa.wesl`

## 2. Fixes

- [x] 2.1 Translucent program (`flags2::TRANSLUCENT`, `light_translucent`)
- [x] 2.2 Fast light loop for `Shd.FastLgt` (`flags2::FAST_LIGHT`)
- [x] 2.3 Shadow and GI taps follow `SdwSmps`; 32-entry tap table; GI window `PassIdx + j`; single plain lookup at one tap
- [x] 2.4 Shared colour slot alpha not squared
- [x] 2.5 Tests: record paths, the shipped foliage's flags, the light buffer header through the plugin, the WESL table against the Rust one, shader validation

## 3. Docs

- [x] 3.1 `docs/RENDERING.md` (Shader builder: shipped variants), crate READMEs
