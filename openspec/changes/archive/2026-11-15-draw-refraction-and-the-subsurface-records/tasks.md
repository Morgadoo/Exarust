## 1. Refraction

- [x] 1.1 The high field (`SceneHighField`), copied from the post chain's bright pass; the binding of each refracting material to its field
- [x] 1.2 `record_path`: `flags2::REFRACTION` and `FRAME_HIGH` (the record's glossiness above 0.85, widened to double); the strength in hundredths in `opacity_falloff.w`
- [x] 1.3 `exa.wesl`'s `refraction`, checked against the expanded `glsclearopq01` and `glsblueopq01` programs (with and without the GI volume)

## 2. Subsurface

- [x] 2.1 `record_path`: `flags::SUBSURFACE` for the subsurface program; the material build's secondary colour, emissive and blood
- [x] 2.2 `exa.wesl`: the plain share without maps, no fall-off or parallax ambient darkening; checked against the expanded `soap01` and `candlemetlylw1` programs

## 3. Fast loop shadows

- [x] 3.1 `light_shadow_fast` and `fast_taps`; three-float `SHADOW_TAPS`; checked against the expanded `lady_fern_a01` program (point light and sun)

## 4. Checks and docs

- [x] 4.1 Tests: record paths, shipped glass and wax, the field binding through the plugin, the strength's hundredths, the read's offset, weight and tint, the WESL table
- [x] 4.2 GPU before/after in one grid (exanima01, glass models), FPS and `picdiff`
- [x] 4.3 `docs/RENDERING.md` (Refraction; Shipped variants), crate READMEs
