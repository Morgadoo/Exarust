## Why

Roadmap task B4 for the surface programs. `exa-tool shader --variants`
listed the shipped program variants `exa.wesl` did not reproduce:
refraction (23 records, 178 draws), the subsurface program for records
other than the race skin (7 groups, about 250 draws) and the fast light
loop's shadow lookups (the translucent foliage and the `FastLgt` plants).

## What Changes

- Refraction (`Mat.Refract`): a lit record in the liquid mode reads a
  field of the previous frame behind it, the post chain's bright pass
  (half size) above a glossiness of 0.85 and its blurred bright pass
  (quarter size, the local reflection's low field) otherwise, pushed along
  the view-space normal by the record's strength (`0x58`, in hundredths,
  in `OpcFll.w`), weighted, tinted by the diffuse colour and mixed into the
  lit colour by one less the alpha (`exa_render::refraction`,
  `flags2::REFRACTION`, `flags2::FRAME_HIGH`). The post chain copies its
  bright pass out beside its blurred one; the material's frame binding is
  `scene_field`.
- Subsurface: every record whose shader word selects the subsurface
  program draws the skin branch with its secondary colour; without a
  diffuse or specular map the plain share is `MatDif`²; no fall-off, no
  parallax ambient darkening, no emissive colour, and the skin's blood.
- Fast loop shadows: each cube lookup offsets by the tap's three floats
  unturned, without the normal offset, against the distance lowered by
  `a²·0.008`; the sun's taps are unturned. `SHADOW_TAPS` in `shading.wesl`
  holds all three floats.
- `exa-tool shader --variants` names the `subsurface` and
  `refraction-low`/`refraction-high` branches.

## Impact

Crates `exa_render`, `exa_assets`, `exa_tool`. Pictures change for the
coloured glass, the gems, the crystal ball and the ornate lamps (refracted
and tinted instead of blended), the candles, soap, ceramics, plant growths
and the demon's skin (scattered light), and the shadows on foliage and
fast-light plants. Simulation and traces are unchanged.
