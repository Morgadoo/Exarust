# Tasks

## 1. Trace

- [x] 1.1 The moon light's colour and intensity (the day record's sun colour, 4.5) and its two lifts
- [x] 1.2 The night fog, ambient and GI sky light (record fog, level ambient, tinted sky cube)
- [x] 1.3 The sun placement's dimmer: the sun light's shadow softness source
- [x] 1.4 The view's post amount: when the post processor is made, where the amount goes, who reads it
- [x] 1.5 The moon shadow on `arenao3`: the fitted map against Bevy's cascades, a top-down view, two moon heights, the caster flags of the level's nodes and props

## 2. Port

- [x] 2.1 `SceneEnvironment::sun_softness_source`; the live frame writes it into `SunShadowRegion::c`
- [x] 2.2 Log the source and the moon at a pinned time; label the post amount as stored, never read

## 3. Proof

- [x] 3.1 Test: the source for a clear night, a cloudier sky and an indoor level, and its hand-over to the region
- [x] 3.2 GPU: arenao1–4 at seed 15, time 0.86, before and after; arena1 before and after; the moon shadow from above at two times and with Bevy's cascades
- [x] 3.3 Docs, spec, READMEs
