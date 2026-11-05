# Tasks

## 1. Trace

- [x] 1.1 The sun map's render routine: depth function, polygon offset, nudge, culling, alpha-tested parts
- [x] 1.2 The caster programs' shader texts (plain and alpha-tested)
- [x] 1.3 The sun's receiving filter in the `lights` template: taps, radius, unbiased depth

## 2. Port

- [x] 2.1 `SunShadowFilter` from the fit, carried in the light records' first element
- [x] 2.2 `sun_shadow` in `shading.wesl`, used by the surface and terrain shaders
- [x] 2.3 The caster state on Bevy's shadow pipeline: offset fragment, culling, alpha test
- [x] 2.4 `EXA_SUN_SOFTNESS` debug override and the filter log

## 3. Proof

- [x] 3.1 Tests: the filter from the fit, the buffer's first element, the caster culling, the hand-over from the fit
- [x] 3.2 GPU: arenao3 seed 15 at noon and dusk before and after; the softness override 0, 1, 3; fence and foliage shadows; frame time
- [x] 3.3 Docs, spec, READMEs
