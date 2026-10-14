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
- Without voxel GI, ambient = environment cube at the normal (mip 5) ×
  level ambient × 16, added to the lights before the diffuse colour.
  **The game always runs with voxel GI** (GI mode 3), whose bounce light
  replaces this term (see Voxel GI). Exarust uses the cube path only outside
  the GI block or with `--no-gi`.
- The cube is `genref01r` for every indoor level (six 256² sRGB faces, each
  with nine mips); outdoors it is `genref02r` masked by `genref02rm`. The
  level record's `EnvMapIA01` is unread.
- Reflections sample the cube along the mirror direction at mip
  (rough − 0.1)·8, as (e + e²) × level ambient × 8 × the geometry term.
- Roughness ("Gloss" in the shaders) depends on the maps present:
  - 1 − glossiness × specular map green;
  - otherwise 1 − glossiness × gloss map red;
  - otherwise 1 − glossiness.

  Glossiness is the record's `0x3C` as stored. Gloss maps are dark, so the
  menu floor's 0.9 becomes a matte roughness of about 0.8.
- The specular colour is zero unless the flags carry `0x4`. It is tripled
  only for PBR materials (type 1) with a specular map.
- Dirt maps only act through vertex colours, which level meshes do not
  have.
- Tangents are generated from the UVs, and the original's bitangent runs
  along −v (its tangent sign is the opposite of MikkTSpace's). Bevy flips
  the MikkTSpace sign too, so its bitangent also runs along −v and the
  normal map's channels are used as stored.
- The program follows the material type:
  - 0 legacy;
  - 1 PBR;
  - 2 unlit;
  - 3 hair;
  - 4 skin (subsurface).
- Legacy materials (type 0) use the older program:
  - no Fresnel per light, and diffuse is N·L;
  - (lights + reflection) × specular colour × (0.5 + 0.5·Schlick);
  - the specular colour is ×3 with a specular map.
- Emissive adds on top. Unlit materials show only their emissive.

**Blending** comes from the material's blend word (`0x44`):
- the source factor is bits 8–11 and the destination bits 0–3;
- `0x100` is opaque;
- ONE / ONE_MINUS_SRC_ALPHA is premultiplied (the diffuse is pre-scaled by
  opacity);
- SRC_ALPHA / ONE is additive;
- `0x4000` is alpha-tested and drawn opaque (cobwebs: alpha = opacity ·
  map², cut at ½).

Blended draws are sorted by the layer in bits 4–7, without depth writes.

## Voxel GI

The original's bounce light is a light volume of 20 cm voxels
(`exa_world::gi_volume` builds it, `exa_render::gi` propagates it).

**Occupancy** (one byte per voxel, on the CPU at level load):
- One block of 256 × 256 voxels across: eight 6 m sectors of 30 columns
  plus an 8-voxel margin on each side. It is 64 voxels high (128 when the
  level's highest and lowest points are more than 12.8 m apart).
- Sectors fall on world multiples of 6 m. The block's top is the level
  record's highest point (`0x98`), truncated to whole voxels.
- The original keeps up to nine blocks resident around the camera; Exarust
  builds one block around the level's centre.
- The top slice holds 26 and the slice below 5, a soft lid.
- Every mesh's stored **GI sample points** (chunk `0x3D0CEC05`: a spacing
  word, a count, then object-space points in cm) are transformed to the
  world and splatted:
  - trilinearly with weight 20, each cell's share rounded on its own, the
    sum saturating at 255;
  - points within a voxel of the block's sides, or in its top slice, are
    dropped;
  - the spacing word is unused. Wall pieces store 100, and their points
    land one per voxel centre, making solid slabs.
- Many wall sets (the menu's `GEWS_UWA02`) store no points, and the original
  generates none for walls. Those walls block nothing.
- **Floor layer**: each tile-grid cell's floor (the floor set's pieces and
  the wall pieces' `+F` strips) is rasterised:
  - one column per 10 cm, taking the highest upward-facing face;
  - the 10 cm cell under that top is marked;
  - every 20 cm voxel holding four marks (its four columns in the same band)
    gets a point at its centre.
  
  A flat floor at height 0 fills the slice from −20 to 0 cm.
- Each column remembers the lowest slice a point fell in. A sector's floor
  is the lowest over its 30 × 30 columns.

**Propagation**, one step per frame (a warm-up runs at load, 600 steps by
default). The light is 8-bit sRGB RGB, with A = 1 − 5·occ.

Only slices from the sector's floor up to the second-highest are updated.
Sectors with no points at all, and the margins, are never updated: they
stay dark and absorb light that reaches them through open walls.

For each updated voxel, with occ normalised:
- open = max(1 − 13·occ, 0);
- for the eight diagonal neighbours n at ±1.33 voxels:
  - a = open(n);
  - P = light(n) · a² (· a with a sun);
  - sum += mix(old, P, a);
- open fades out over the lowest slices: · (1 − 2^(−16·r));
- k = min(8·Σa, 1) · (0.3·open + 0.699);
- light = sum·k/8 + base·open + Σ lights;
- base = level record `0x30` (as stored) × 0.001;
- each light adds lin(colour)·intensity·0.15·0.01 · (1 − d/R)⁴ · open.

Light builds up toward about direct/(1 − k). With open walls and dark
sectors around it, the menu room converges within a few hundred steps.

**Open question:** with every input verified against the executable, the
menu still renders darker than the original's capture. The floor's bounce
light is about 0.005 linear, and the torch-lit walls are about 1.45× darker
even without GI, so part of the gap lies outside the GI. This needs a
side-by-side check on the GPU at the same camera.

**Shading**:
- Four taps 20 cm out along the normal, spread across the tangent plane,
  averaged. Light gives g, alpha gives the occlusion o.
- a = |g|; b = (1 − a)²·0.03/(a + 0.016) + 0.47a + 0.53, which lifts faint
  light.
- ambient = g · lin(level ambient) · 15 · b · o.
- Reflections are the cube along the mirror direction at mip
  (rough − 0.05)·8 × ambient × 8 × 2 × the geometry term.

## Lights

- A point light's shader colour is linear(colour) · intensity · 2/π. Exarust
  sets Bevy's intensity to 8 lumens per unit, since Bevy divides by 4π.
- Attenuation is max(1 − 0.8·d/R, 0)⁴, reaching nothing at 1.25 radii.
  Exarust sets Bevy's range to 1.25·R and computes (1 − d/range)⁴ itself.
- Lights cast shadows from cube maps.
- The sun's colour per unit of intensity is 160 lux in Exarust's scene units
  (scaled back in the shader).

## Fires

A model node with a fire emitter (`0x3DD0E002`, size s; a torch's `_Fire`
is 0.2) becomes an omni light:
- colour sRGB (1, 0.68, 0.43);
- intensity 8√s and radius 1500√s cm, ×1.25 for 0.2 < s ≤ 0.35; larger
  fires use other templates;
- placed 5 cm above the node, casting shadows.

Every tick the intensity is density · (1 + 0.05·x) · base:
- x is a soft-bounded random walk. With u uniform in [0, 1): v −= u past
  +1, v += u past −1, otherwise v += 2(u − ½); then x += 0.3·v·u.
- The density ramps in as 0.8·density + 0.2·(live particles / maximum).

Candles have no fire and give no light. The flame itself is premultiplied
particles with a nine-key colour ramp (not done yet).

## Cameras

- Vertical field of view in degrees.
- Main menu: 60°, eye (0, 5, 7) m looking at (0, 0, −2) m.
- Character creation: 50°.
- Play: 45°·√(1.6/aspect) at the reference zoom, clamped to 10°–100°.

## Not done yet

Flame particles, the sun and terrain in the GI volume, several resident GI
blocks, fog, displacement and parallax, vertex
dirt, glass refraction, effect programs (smoke, powers), hair and skin
programs, terrain on the new shading, and the outdoor environment cube.
Software rendering here cannot afford the light shadows, so test frames use
`--no-shadows`.
