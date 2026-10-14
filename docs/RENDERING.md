# Rendering

How the original draws a frame, and how Exarust rebuilds it on Bevy
(`exa_render`). Status: **verified** in the executable (REA on the renderer
and the decoded shader programs). The `.flb` shader libraries are
obfuscated by a per-word rotation keyed by an LCG; the decoded programs stay
in the git-ignored `context/`.

## Colour

The original works in **linear light with sRGB storage**:
- Material, light and environment colours are linearised on the CPU (the
  exact sRGB transfer function).
- Colour textures upload as sRGB, and data textures (normal, gloss, height,
  packed specular) as linear. The `.rfi` format word's top byte decides:
  `0x81`/`0x01`/`0x02` colour, `0x82`/`0x92`/`0x12` data.
- No shader applies a gamma power.
- The scene renders into an 8-bit sRGB target, so the lit result **clamps
  at 1**. There are no float buffers and no automatic exposure.
- Exposure is a constant 1. The settings slider scales lights, ambient and
  emissive. There is no gamma or brightness option.

## Post-processing

Always on (`exa_render::post`, replacing Bevy's tonemapping; cameras get
`Tonemapping::None`):

1. **Bright pass** at half resolution: (c + c²)·0.8, clamped.
2. **Blur**: separable, 31 taps (a centre and 15 pairs, 2 half-resolution
   texels apart), horizontally then vertically at a quarter of the
   resolution. Weights fall from 0.0711 to 0.0162; the total gain is about
   1.23.
3. **Final pass**: the scene clamped to 1, optionally desaturated (0 in
   play), then
   - the **grading curve** b = 0.9c² + 0.1c, c′ = 7.05b/(6.5b + 0.55), a
     strong lift of the mid-tones (0.2 → 0.43, 0.5 → 0.83, 1 → 1);
   - the **bloom** combined like a screen:
     1 − (1 − c′)(1 − (1.75·s·max(B − 0.03, 0))²), with strength s = 1.
4. Output through the sRGB encoding.

The ACES curve found among the shader programs is unused.

## Surfaces

`exa_render::material::ExaMaterial` with `exa.wesl` reproduces the main lit
program.

**Material record** (the first 184 bytes of a material payload):

| offset | field |
|---|---|
| `0x00` | emissive colour |
| `0x10` | diffuse colour (×1.5 for legacy materials, type 0) |
| `0x20` | specular colour |
| `0x38` | opacity |
| `0x3C` | glossiness |
| `0x40` | specular level |
| `0x44` | render class (low nibble 3 = transparent; bits 12–14 = blend mode) |
| `0x48` | flags: lit `0x3`, two-sided `0x10`, refraction `0x200`, opacity falloff `0x800`, plus the texture slots |
| `0x50` | type: bit 0 means the specular map is a packed intensity/gloss/metal map |

**Texture slots**:
- `0x20000` diffuse
- `0x40000` specular
- `0x80000` emissive
- `0x100000` environment override
- `0x200000` normal
- `0x400000` **opacity** (its alpha; cobwebs use opacity · map²)
- `0x800000` height
- `0x1000000` gloss
- `0x8000000` dirt (likely)

**Lighting**:
- GGX specular with r² = rough⁴, k = (rough + 1)²/8, rough = 1 −
  glossiness·gloss map. Schlick-style Fresnel F0 + (1 − F0)·2^((−5.55473h
  − 6.98316)h). No 1/π.
- Diffuse is N·L·(1 − F), times the diffuse colour.
- Packed specular maps give the metalness: diffuse × (1 − metal), F0 =
  mix(0.04, albedo·specular, metal), and the intensity scales the specular.
- Ambient = environment map (normal, blurred) × level ambient × 16, added to
  the lights before the diffuse colour. Exarust uses a constant average for
  the map until environment maps are decoded.
- Emissive adds on top. Unlit materials show only their emissive.

## Lights

- A point light's shader colour is linear(colour) · intensity · 2/π. Exarust
  sets Bevy's intensity to 8 lumens per unit, since Bevy divides by 4π.
- Attenuation is max(1 − 0.8·d/R, 0)⁴, reaching nothing at 1.25 radii.
  Exarust sets Bevy's range to 1.25·R and computes (1 − d/range)⁴ itself.
- Lights cast shadows from cube maps.
- The sun's colour per unit of intensity is 160 lux in Exarust's scene units
  (scaled back in the shader).

## Cameras

- Vertical field of view in degrees.
- Main menu: 60°, eye (0, 5, 7) m looking at (0, 0, −2) m.
- Character creation: 50°.
- Play: 45°·√(1.6/aspect) at the reference zoom, clamped to 10°–100°.

## Not done yet

Torch and flame lights, environment maps and specular reflections, the
voxel GI volume, fog, displacement and parallax, dirt, glass refraction,
effect programs (smoke, flames, powers), hair and skin programs, and
terrain on the new shading.
