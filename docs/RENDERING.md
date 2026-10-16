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
  `0x81`/`0x01`/`0x02` colour, `0x82`/`0x92`/`0x12` data. A loader can force
  data: the race's skin textures load that way (see Skin).
- No shader applies a gamma power.
- The scene renders into an 8-bit sRGB target, so the lit result **clamps
  at 1**. There are no float buffers and no automatic exposure.
- Exposure is a constant 1. The settings slider scales lights, ambient and
  emissive. There is no gamma or brightness option.

## Supersampling

At the original's 4× setting (`Supersample = 4`, level n = 3) each frame is
drawn four times at window resolution:
- each sub-frame has a sub-pixel jitter of (−0.29, −0.15), (0.30, 0.15),
  (0.20, −0.35) or (−0.21, 0.35) pixels;
- the four go into the quadrants of a buffer twice the size;
- they are averaged with equal weights into the window-sized image, in
  linear light.

Bloom and the final pass then run at window resolution. Supersampling also
turns on the final pass's sharpen.

Exarust (`exa_render::supersample`) draws the scene at twice the window's
resolution on each axis, an ordered 2 × 2 grid instead of the jittered
four. A display camera resolves each 2 × 2 block by bilinear sampling at
its centre, then runs the post chain at window resolution. It is on in the
viewer; `--no-supersample` turns it off. The original multisamples nothing
(`HWMultisample = 0`), so the supersampled scene is drawn without MSAA.

## Post-processing

Always on (`exa_render::post`, replacing Bevy's tonemapping; cameras get
`Tonemapping::None`):

1. **Bright pass** at half resolution: (c + c²)·0.8, clamped.
2. **Blur**: separable, 31 taps (a centre and 15 pairs, 2 half-resolution
   texels apart), horizontally then vertically at a quarter of the
   resolution. Weights fall from 0.0711 to 0.0162; the total gain is about
   1.23.
3. **Final pass**: the scene clamped to 1, then:
   - a **sharpen**, on when supersampling is (strength 1.5 × (0.075·n +
     0.4) at supersample level n; 0.9375 at the original's 4×). It takes
     four diagonal samples half a pixel out (each a 2 × 2 average) and
     their contrast a = 16·Σ|C − S|². The weight is (min(0.16a, 0.8) −
     clamp(1 − a, 0, 1)¹⁶)·strength, and C′ = C + w·((ΣS − C)/3 − C): an
     unsharp mask on flat areas, easing toward the neighbours on strong
     edges;
   - optional desaturation (0 in play);
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
| `0x48` | flags: lit `0x3`, specular `0x4`, emissive `0x8`, two-sided `0x10`, refraction `0x200`, opacity falloff `0x800` (the loader also sets it when `0x64` > 0), plus the texture slots |
| `0x4C` | extra flags: `0x100` the opacity fall-off fades toward the outline (most records), `0x100000` displacement shadows |
| `0x50` | type: bit 0 means the specular map is a packed intensity/gloss/metal map |
| `0x64`, `0x68` | opacity fall-off amount and exponent |

**Texture slots**:
- `0x20000` diffuse
- `0x40000` specular
- `0x80000` emissive
- `0x100000` environment override
- `0x200000` normal
- `0x400000` **opacity** (its alpha; cobwebs use opacity · map²)
- `0x800000` height
- `0x1000000` gloss
- `0x8000000` dirt (the dirt modes' texture)

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
  have. Dirt mode 4 is wound blood ([Blood](#blood)).
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
- **Emissive** (MatEmi) is lin(emissive colour), doubled for materials
  with lit or specular bits and an emissive layer (torch handles, lamp
  bodies). It adds on top. With an emissive map the PBR program weights
  it by N·V²·z + 1 − z, z the fall-off's amount term (0 without a
  fall-off).
- **Unlit** materials (no lit or specular bits) show only their emissive:
  colour MatEmi × the emissive map, alpha the opacity (`0x38`, not the
  colour's alpha), plus the fall-off when they have an emissive map.
- **Opacity fall-off**, for every record whose amount `0x64` exceeds 0.01
  in magnitude: alpha += 2^((x − N·V)·y)·z, with f = 2^(−1.8·`0x68`).
  The original packs a = (f + 1)·`0x64` and y = 1.8·`0x68`, both negated
  with extra flag `0x100`; z = |a|, and x = 1 when a is negative, else 0.
  With the usual positive amount and the flag the surface fades toward its
  outline; without the flag, toward its centre. The opacity loses
  (f + 1)·f. On lit surfaces only the PBR program adds the term (and
  weights the emissive map, above); the legacy program's lit path does
  neither, though no legacy lit record carries a fall-off. The menu
  salve's glow (`¹jarsm01glow`: unlit, opacity 0, amount 1, exponent 2)
  has alpha 1.08 where it faces the eye and 0.09 at its outline. Every
  additive material in the corpus is such a glow (salves, power crystals,
  light-globe glass, teleporters, lamps).
- Emissive surfaces light nothing else: the GI takes no emissive term. A
  glowing object spreads light through a voxel light (see Voxel GI) or a
  point light in its model.
- **Parallax** (`0x800000` height layer, with a normal map; shading quality
  ≥ 4):
  - Depth: P = round(50·L), where L is the height layer's amount (body +4;
    0.6 on the menu floor). With the height map's size W × H,
    Y = round(P·2048/H), X = round(Y·H/W), and the scale is (X, Y)·3·10⁻⁵:
    a fixed number of texels.
  - Shift: the view from the eye in tangent space (T, +∂P/∂v, N) gives
    d = view.xy·scale·(4 + 3·view.z).
  - Four accumulating steps uv −= d·(h(uv) − ½). Every map reads at the
    final coordinates.
- **Displacement shadows** (material extra flags `0x4C` bit `0x100000`;
  shading quality ≥ 5): each light is dimmed by height taps marched toward
  it in the tangent plane.
  - Step a = (`0x5C`·0.004)·(2 − l.z), with l the light in tangent space.
  - lit = clamp((h₀ + 0.1·l.z − h)·20, 0, 1), with h₀ = h − ½ +
    `0x60`·0.05 + ½.
  - The taps are averaged: four per pass, sixteen in all over the original's
    four supersampling passes.
- Without displacement shadows, parallax darkens the ambient by
  min(h − ½ + 1.2, 1).

**Textures** are sampled with the original's settings at its top texture
quality (`TextureQuality = 5`) and 4× supersampling:
- anisotropic filtering 2^(4 − mode), so 16× for ordinary textures;
- a LOD bias of base × f(supersample), with base −0.75 at quality 5 (−0.5 at
  4, 0 below) and f = (ss − 1)/3 for ss ≥ 2. That gives −0.75. It applies
  to colour textures (diffuse, emissive, opacity, legacy specular), not to
  normal, gloss or packed specular maps. wgpu samplers have no bias, so the
  shader applies it.

Uncompressed RGB8 and RGBA8 textures reach the shaders red first, as
stored, raw or packed (`formats/RFI.md`). The original uploads them as RGB
and RGBA, so the embers' glow map is orange, foliage green and RGB8 normal
maps keep x and y in red and green.

Floor UVs are generated per cell (see `formats/RFC.md`, tile grids): one
square repeat per 6 m cell.

**Terrain** shades with the same lighting as models (`exa_render::shading`,
shared by `exa.wesl` and the terrain shader), as the original's terrain
program uses its `lights`, `ambient` and `speculargi` includes. The original
composites each 6 m block of a tile into colour, normal, gloss and height
textures. Exarust evaluates the same per-layer stacking per pixel:
- texture coordinates (X, −Z)·f0/12 per metre;
- colour: texture × linear(tint) × 1.5·min(f9, 1), cover f9, sharpness
  f2/f3; then × 0.75 (the terrain's `MatDif`);
- normal: (N·2 − 1)·f11, mixed by the mask-sharpened opacity (f4/f5), in a
  frame of +X and +Z across the ground (green along −v);
- gloss: the gloss texture × f10 × 1.5·min(f9, 1), stacked like the colour;
  roughness = 1 − gloss (no wetness indoors);
- parallax height: f20 + (B − f21)·f22 from the bump slot, stacked like the
  normals;
- parallax: scale 0.007 in 6 m block units, four accumulating steps, then
  eased back toward the unshifted coordinates by min((h₀ − h₃)·2, 1);
- Fresnel F0 = 0.04 per light; GI or cube ambient, cube reflections;
  specular × min(255·r, 1).

The original's composites hold about 170 texels per metre and have no mip
levels, so its terrain always shows the layers' full detail. Exarust reads
the colour and normal layers at their base level and lets supersampling
smooth the aliasing.

Each layer's height, gloss and blend mask are packed into one texture, to
fit the per-stage texture limit. The terrain's displacement self-shadowing
is effectively off in the original (its step is ~0).

**Blending** comes from the material's blend word (`0x44`):
- the source factor is bits 8–11 and the destination bits 0–3;
- `0x100` is opaque;
- ONE / ONE_MINUS_SRC_ALPHA is premultiplied (the diffuse is pre-scaled by
  opacity);
- SRC_ALPHA / ONE is additive. Exarust draws it through Bevy's
  premultiplied blending as (colour·alpha, 0), colour clamped to 1 first;
- every fragment's alpha is clamped to [0, 1] before blending, as the
  original's 8-bit target clamps it. The opacity fall-off takes it past 1
  at a glass's outline, which on Exarust's float target would otherwise
  subtract what is behind;
- `0x4000` is alpha-tested and drawn opaque (cobwebs: alpha = opacity ·
  map², cut at ½).

Blended draws are sorted by the layer in bits 4–7, without depth writes.

### Skin

The body model's own skin material (`skinhuman`, local to `humanbase.rfc`
and `ancientbase.rfc`) is never drawn. For races whose record asks for it,
the original puts a built-in material in its place (`exa_assets::human_skin`):
`_HumanSkin`, or `_AncientSkin` for ancients. Both have diffuse, specular
and glossiness 1, PBR with a packed specular map and textures generated
per character; they differ in the subsurface colour, sRGB (1.0, 0.8, 0.46)
for humans and (1.0, 0.63, 0.133) for ancients (read from the two built-in
payloads), and in the blood-spatter dirt only humans get. Exarust tells
ancients by the body model `ancientbase.rfc` (the original by a race kind
word it sets on the character, 0x200 for ancients) **[inf]**.

- **Textures**: the race's own, which the race loader names after the
  body model's stem and loads as data: `<stem>difm` (colour), `<stem>spcm`
  (gloss) and `<stem>nrmm0…5` (normal, one per physique), with `f` in place
  of the last `m` for women. Whatever the skin material names plays no
  part: `humanbase.rfc`'s names the same textures, `ancientbase.rfc`'s only
  a `missing.dds`. Exarust uses the men's textures and the first physique's
  normal map.
- **Diffuse map**: the race's skin texture (`humandifm`, `humandiff` for
  women) times the character's skin tone. The original loads the race
  texture as data, so the product is taken on its stored, sRGB-encoded
  values; it renders the map with the sRGB encoding off into an sRGB
  texture, which drawing the skin decodes once. The tone multiplier comes
  from a 3 × 3 grid read bilinearly at the appearance's two tone values:
  0.29–0.64, (0.46, 0.47, 0.47) at the centre. The texture's own skin
  colour, sRGB (0.80, 0.65, 0.57), shows as linear (0.112, 0.076, 0.059) at
  the middle tone. Exarust has no skin tones yet: every human and ancient
  wears the middle one.
- **Specular map**: intensity 1, gloss 0.8 × the race's gloss texture
  (`humanspcm`), no metal. So F0 is 0.04 and the roughness 1 − 0.8·gloss.
- **Subsurface program**: of the diffuse colour, the subsurface colour's
  share (linear (1.0, 0.604, 0.178) for humans) scatters and the rest is
  plain diffuse, N·L without the Fresnel share. The scattered share is lit by a
  wrapped term on the normal map read at mip 2: with b = N·L and
  c = 0.5 + 0.05·2, min(max(b + c, 0)²/2, ½) + max(b − 1 + c, 0). It takes
  its colour from the diffuse map at mip 1.5. Ambient reaches both shares.
  Lit skin is as bright as plain diffuse (1.10, 1.06, 1.02 facing a light),
  with a red glow past the terminator (0.18, 0.11, 0.03 at N·L = 0).
- There is no light-receive scale, skin cap or exposure adaptation: lit skin
  is limited only by the 8-bit scene clamp.

Not yet: skin tones per character (the player's are in its save, which is
not shipped), women's skin textures, the generated normal map's blend of
the physique variants (`humannrmm0…5`), the dirt layer. Blood: see
[Blood](#blood).

### Blood

Wound blood is neither a texture nor a decal in the original: it is one
byte per vertex, the alpha of the mesh's colour stream (255 clean),
lowered around each wound and never raised, and a shader mode, **dirt
mode 4**, that reads it against a shared tiling speckle texture.

**Channel** (`exa_character::blood`): Exarust stores it as the colour
stream in bytes (Unorm8x4, white, the blood in alpha). A mesh without one
counts as white. A model's meshes are shared by every character wearing
it; the first wound that lowers a value of one gives the struck character
its own copy (`Bloodied`), which later wounds paint in place. A whole
human body in braies costs 278 KiB (20 meshes, 2 989 vertices), the
duellist's outfit 1.9 MiB (145 meshes, 20 668 vertices), on the CPU and the
GPU each (76 bytes a vertex, 4 bytes an index); clean characters copy
nothing.

**Targets**: a character's skinned meshes (its body and every skinned
garment, which the original merges into one skinned mesh) and every other
mesh under its head part (hair, helmets). Never held items, rigid items on
other parts, or body parts hidden under apparel.

**Wounds from hits** (`exa_combat::wounds`): a blow bleeds when the
struck character's race bleeds and the wound's size R (the blow's lasting
damage on the part, its multiplier applied, before the red bar's 0.75)
exceeds 0.05 ([formats/OTHER.md](formats/OTHER.md), Damage). The wound
waits on the victim, seven at most (a new one is dropped while seven
wait), with the striking body (the blade, or the fist), where on it the
blow landed (weights in its tetrahedron: Exarust's blades move rigidly, so
the tetrahedron is the blade's origin and the ends of its axes), the
contact point and R. Every body tick counts each waiting wound down; seven
ticks after the blow (84 ms) the first is painted, one a tick, on corpses
too. Its travel D is where the striker's weights put that point now less
the contact point, its radius 30·R + 10 cm (11.5 cm at the threshold, 19 cm
at R = 0.3). Nothing paints the striker (a fist stays clean). Blood never
spreads, fades or drips, and corpses keep it.

**Painter**, for a wound with contact point P, travel D (how the striking
point moved on since the blow) and radius r (the original's 30·R + 10 cm),
on the vertices' current skinned positions:
- centre c = P − D/2; d = v − c;
- e = |d|²·(1 − cos θ) when d points ahead along D (cos θ > 0), else |d|²:
  a ball of radius r behind the centre, ahead a smear about √2·r wide that
  runs on to the end of the mesh (e ≈ ρ²/2 at a distance ρ from its axis);
- where e ≤ r²: f = 1 − e/r², u uniform in [0, 1), g = f + (1 − f)·u, value
  255·(1 − f·g²), rounded to the nearest byte, written only when lower.
  0 on the axis, 128–223 at f = ½, 191–251 at f = ¼.

**Shading** (`exa_render::material::BloodProgram`, `exa.wesl`): a material
with the mode binds `spatter01` (`Resource.rpk`, 1024², BC4, one channel)
as its dirt map. With a the speckle at UV×2 (UV×1 on hair) and α the
interpolated vertex alpha:
- blood amount da = clamp((a − α)/a·k, 0, 1): k = 1.3 on cloth (the
  original's apparel and item program), 1 on skin, where α is read no lower
  than 0.02, and on hair. The speckle's texels at or above 0.5 cover 13 %,
  at or above 0.25 46 %, at or above 0.125 83 %: a lightly painted vertex
  shows separate bright droplets, and the mottled background fills in as α
  falls toward 0;
- the diffuse colour (both shares of the skin) is multiplied by, clamped
  to [0, 1]:
  - skin: (0.6, 0.03, 0)·(1 − da²/2)·(1 − 0.3·da) + (1 − da), (0.21,
    0.0105, 0) at da = 1;
  - cloth: (0.6, 0.03, 0)·(1 − 0.3·da) + (1 − da), (0.42, 0.021, 0);
  - hair: (0.45, 0.03, 0.02)·(1 − 0.3·da) + (1 − da);
- wet sheen (roughness): skin loses da·(0.1 + 0.6·a), no lower than 0.1;
  cloth moves toward 0.2 by da²·(0.5·a + 0.5), hair by da²·(0.7·a + 0.3).
- Meshes without a colour stream skip it all, so clean characters draw
  exactly as before.

**Materials with the mode**: the human skin (skin program; the ancient
skin has none); every painted or run-time (`&`) apparel material and every
factory item (cloth; the original's `_procmat1` and `_procmatr`); any
record whose extra flags (`0x4C`) select dirt mode 4 (bits `0x7000` =
`0x4000`) and that has a diffuse map, by its type (1 cloth, 3 hair, 4 skin;
types 0 and 2 have no dirt modes); and the body-model materials whose names
start with `s` of a race that bleeds (`races.rdb` flags bit 0: the race
loader turns the mode on for `skinogre`, `sdemona01`, `shrtogra`,
`sknhrtghoula` and the like, not the eyes). Exarust draws no hair yet.

**Evidence**: reverse engineering of `Exanima.exe` 0.9.5 (the painter's
constants and store, its targets, the white fill of meshes without
colours, the race loader's `s` test), the game's shader sources (the skin,
apparel, hair and translucent programs' dirt mode 4 blocks), the corpus
(`races.rdb` flags, the race models' materials, `spatter01`'s histogram,
pinned by tests), and retail screenshots: blood pixels median sRGB (54, 25,
21) on skin of about (81, 68, 58), speckled edges, glossy thick coats.

In the arena duel the struck fighters' blood pixels have a median sRGB of
(64, 31, 26) on the living player and (51, 25, 22) on its corpse, against
the original's (54, 25, 21) and (47, 24, 20).

Not yet: blood sprays, blade stains, the attacker's own blood when it
strikes with a body part of its own (creatures), the hair program, the
translucent program's blood ((0.5, 0.03, 0)), and dirt modes 1–3. Exarust
never re-dresses a character, so the original's clean meshes after an
equipment change do not arise yet.

## Voxel GI

The original's bounce light is a light volume of 20 cm voxels
(`exa_world::gi_volume` builds it, `exa_render::gi` propagates it).

**Occupancy** (one byte per voxel, on the CPU at level load, built on the
async compute pool: a large outdoor level's block takes seconds, and the
window keeps answering meanwhile; the level's points and triangles are put
in world space there too):
- One block of 256 × 256 voxels across: eight 6 m sectors of 30 columns
  plus an 8-voxel margin on each side. It is 64 voxels high (128 when the
  level's highest and lowest points are more than 12.8 m apart).
- Sectors fall on world multiples of 6 m. The block's top is the level
  record's highest point (`0x98`), truncated to whole voxels.
- The original keeps up to nine blocks resident around a point 20 m in
  front of the camera. Exarust keeps one block centred on that point, and
  rebuilds it (with a new warm-up) when the point moves more than two
  sectors. One build runs at a time; a newer focus waits for it. The
  level's points and floor triangles are collected once in world space.
  When a load ends, the block is rebuilt around the camera's starting
  view if it is not already there.
- The top slice holds 26 and the slice below 5, a soft lid.
- Terrain comes first: in each interior column (not the margins), voxels
  below the terrain surface hold 20, and the surface voxel its covered
  fraction of 20.
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

**Propagation**, one step per frame. A new block first gets a warm-up,
600 steps by default, run 16 steps a frame (all of them in one frame stalled
the window for seconds); the loading screen waits for it. The light is 8-bit sRGB RGB, with A = 1 − 5·occ.

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
- each light adds lin(colour)·intensity·multiplier·0.15·0.01 · (1 − d/R)⁴
  · open · shaping;
- the lights injected are those whose reach overlaps the block, nearest its
  centre first, up to 32. The original injects each sector group's lights,
  up to 12 per group.

**Voxel lights** (`0x3D0C` nodes: glowing items and a few level nodes)
add straight into the volume, after the propagation, every step and in
the warm-up:
- colour C = lin(rgb)·√intensity·3 (the original's linear workflow; 0.6
  on its legacy path), × 0.1 per step. An item whose model section sets
  bit 0 scales the intensity of the voxel lights under its root by
  k = (1 − c/255)², c its placement's condition byte, and the emissive
  colour (not its alpha) of the meshes under its root by the same k, so a
  spent salve's glow dims with its light;
- the voxels reached: 6 × 6 columns around the voxel corner nearest the
  light, never in the 8-column margins, and the slices from 3 below that
  corner to 3 above (k + 1 slices from k − 3 when the corner k is below
  slice 6; none at or below the bottom);
- each voxel gains C·0.1·atn·0.25·sdw, with atn = max(1 − 5000·d², 0)⁴, d
  in texture units (the vertical counted in voxels across, 256), which
  reaches nothing at 3.62 voxels (72 cm);
- blocked = min(20·(1 − o(voxel)·o(midpoint)), 1) with o = 1 − occupancy,
  and sdw = 1 − blocked·(1 − max(2 − 0.75/atn², 0)): through geometry the
  light reaches only about 1.2 voxels;
- alpha is left alone, and the stored radius is unused. Exarust adds the
  term only where the step writes, keeps up to 64 voxel lights per block,
  nearest its centre first, apart from the point lights.

The menu's Healing Salve adds (0.011, 0.125, 0.30) per step at its own
voxel, 1.7 times a torch's blue over its box, packed into a 70 cm ball:
the blue on the shelf and the wall behind it.

Light builds up toward about direct/(1 − k). With open walls and dark
sectors around it, the menu room converges within a few hundred steps.

**Menu check:** at the original's menu camera, with shadows, after the GI
has warmed up (`--frames 420`; at the default 90 frames the GI block is not
built yet), the mean brightness of the torch-lit walls and floor lies
within about 4 of 255 sRGB levels of the original's capture. Before
blended cobwebs cut their shadows (see Lights) the same regions were 10 to
22 levels darker, which is most of what earlier notes reported as a darker
menu.

**Shading**:
- The shadow filter's disc taps (below), turned the same way, spread half a
  unit across the normal and taken 20 cm out along it, averaged. Light
  gives g, alpha gives the occlusion o.
- a = |g|; b = (1 − a)²·0.03/(a + 0.016) + 0.47a + 0.53, which lifts faint
  light.
- ambient = g · lin(level ambient) · 15 · b · o.
- Reflections are the cube along the mirror direction at mip
  (rough − 0.05)·8 × ambient × 8 × 2 × the geometry term.

## Lights

- A point light's shader colour is linear(colour) · intensity · 2/π (not the
  multiplier). Exarust sets Bevy's intensity to 8 lumens per unit, since
  Bevy divides by 4π.
- Attenuation is max(1 − 0.8·d/R, 0)⁴, reaching nothing at 1.25 radii.
- The light record (body): flags, sRGB colour, 4 unread bytes, then:
  - the **multiplier**, which scales only the light's GI injection (0 leaves
    the light out of the GI);
  - the **intensity**, which scales its colour;
  - the radius in cm;
  - one unread value;
  - the **shadow softness**.

  Nodes whose flags (`flags2`) masked by `0x300` equal `0x200` add the spot
  values q, p. Node flag `0x40000` makes the light fall off with distance;
  without it, it reaches everywhere at full strength.
- Lights with spot values are shaped vertically:
  - y = ½·sin(qπ/2) for q ≥ 0, otherwise 3 − 3/cos(qπ/2); z = p + 1;
  - factor max((2^(z·(Ly + (1 − Ly)·y)) − 1)/(2^z − 1), 0), with Ly the
    vertical part of the direction to the light (1 straight below it);
  - applied on surfaces and in the GI injection.
  
  Exarust keeps y and z in a per-light record the shaders read
  (`exa_render::lights`). Each light's id travels in its depth-bias field,
  which Exarust's own shadow filter does not use.
  Exarust sets Bevy's range to 1.25·R and computes (1 − d/range)⁴ itself.
- Lights cast shadows from cube maps (512² per face; the sun's map is 4096²
  at the top shadow detail).
- Surfaces with opacity ≤ 0.8 (glass, decals) and those whose record has
  extra flag `0x4` cast no shadows. The others cast whole, except through
  the original's **alpha-tested caster**, which is used for:
  - alpha-tested materials (blend word `0x4000`), with or without a map
    that gives alpha;
  - materials with the opacity flag and an opacity above 0.75, whatever
    their blending. The flag is `0x400000` in the record's flags, which
    the original also sets when an opacity map loads. Records with the
    flag and records with an opacity layer differ in 61 of the corpus's
    1584 materials. The menu's cobwebs (`cobweb01`) are alpha-blended,
    opacity 1, with both the flag and the layer.

  The alpha-tested caster draws both sides of the surface. It drops every
  texel whose cut texture has an alpha below ½: the opacity map, otherwise
  the diffuse map. Unlike the colour pass, the cut ignores the material's
  opacity and does not square the opacity map. This rule holds in the
  cached cube maps (static casters of point lights); Bevy's own shadow
  passes (the sun, skinned meshes) still cut as the colour pass does
  (`exa_prepass.wesl`).
- A light leaves its own object out of its shadow map: its siblings, its
  parent and its grandparent. A fire's light hangs under the fire node, so a
  torch does not shadow its own flame. Only the torch's hidden `TorchSdw`
  disc casts, a narrow cone along the handle. A light at its object's root
  excludes nothing. The excluded meshes still cast into other lights.
- **Shadow links**: a mesh casts into a light only when the two are linked,
  as the original links nodes to lights:
  - their bounding spheres overlap (the light's of radius R);
  - (1 − 0.8·d/R)⁴ · luma₆₀₁(linear colour · intensity) > 0.01, with d the
    distance from the light to the mesh's sphere;
  - the mesh is not in the light's own object.

  Links are kept current incrementally: a moved light is relinked against
  every mesh, a moved mesh against every light. Exarust hands them to Bevy
  as render layers: each shadowed light has a layer of its own (from 2 up;
  layer 1 is the supersampling resolve quad), a mesh carries layer 0 and its
  lights' layers, and the scene cameras see them all. In exanima01 (229
  lights, 13 000 meshes) this keeps memory flat at about 3 GB. With every
  mesh in reach casting into every light, Bevy's per-face bookkeeping grew
  to about 12 GB and the frame rate fell below 2 FPS.

- **Lights per mesh**: a mesh is lit only by its 6 strongest links, as the
  original lights an object. A mesh with no links gets no point light, only
  ambient. Each mesh's list sits in the lights buffer, and its slot travels
  in the mesh tag above the shadow-bias level. Terrain tiles, which are
  larger than the original's 6 m terrain blocks, and meshes that cast no
  shadows keep every light in reach.

- **Lights stop at walls** (tile-grid levels, ranged lights): each light
  floods the 6 m cell grid from its own cell, top down, carrying an angular
  window of the directions still open:
  - the light's own cell sees everything; each open side of it starts a
    half-plane window;
  - the walk is depth-first and stops when 31 entries are waiting;
  - a cell is entered while its centre lies within R/6 m + ½ cell of the
    light, measured top down;
  - the walk only leaves a cell through sides facing away from the light;
  - a side narrows the window to its opening. A solid wall closes it. A wall
    stub (edge bit 1 or 2) moves an end to ±0.2 cell, a corner flag moves an
    end to ±0.4 cell, and a door is a fixed ±0.2 cell (±0.35 in cells with a
    special tile), whatever its real width and whether it is open or shut;
  - each cell records the window it was entered through (re-entries widen
    it), for at most 12 lights; later lights are dropped there.

  A ranged light links to a mesh only if the mesh's angular extent (centre
  angle ± atan(r/d)) overlaps the window in the mesh's cell, or in a
  neighbouring cell its box spills into past no wall; a mesh around the
  light always passes. That keeps light and shadow casters in their rooms.
  Terrain tiles, far larger than the original's 6 m terrain blocks, skip
  the test. The original's GI also takes its lights from the cell lists;
  Exarust's GI still picks the lights nearest its block.

- **Cached cube shadows** (`exa_render::cube_shadows`): static meshes cast
  into cube maps kept across frames, as the original keeps its shadow maps:
  - a face is rendered only when it is needed (its pyramid out to R meets
    the camera's view) and dirty (the light moved, a caster linked to it
    moved or changed links, or the light just took its slot);
  - torch flicker changes only colour and intensity, so it never dirties a
    face;
  - each face holds the original's linear depth, distance / (R·k_c), in
    16 bits, with near plane 3 cm and back faces culled. Two-sided
    materials and the alpha-tested caster (see above) draw both sides;
  - a face the renderer could not draw stays dirty and is drawn later,
    because a face never drawn reads as wholly shadowed. That happens on
    the first frame, or while a pipeline or mesh is not ready;
  - receivers compare 0.995·d / (R·k_r), lit when less. With k_c and k_r
    from both sides' shadow-bias levels, this is the original's rule
    exactly, cross terms included;
  - the 64 cubes form a pool. A light that needs a slot takes a free one,
    or one whose light needs no face this frame (that light's faces go out
    of date).

  Skinned meshes (characters) still cast through Bevy's cube maps, which
  are re-rendered every frame. Bevy's map is kept only for lights that have
  a skinned mesh linked. The shaders multiply both. Skinned meshes carry
  per-joint boxes (`DynamicSkinnedMeshBounds`), so they are culled per view
  and per cube face.

  Not done yet: draw sets per room, with one portal step into the next
  room. The notes are in `context/re/optimizations.md`.
- **Filter** (point lights):
  - the surface is pushed toward the light by 0.008·(1 − |N·L|)·d along the
    normal;
  - eight taps from a fixed disc lie around the light direction, turned per
    pixel by π·fract(3.5557·x + 0.3093·y), each a hardware 2 × 2
    comparison, averaged;
  - the disc radius is (softness + 1)·0.005: 0.0075 for level lights
    (softness 0.5), and (2j + 1)·0.005 for fires, where the jitter j walks
    by ±0.0625 a tick within [0.87, 1.15];
  - distances are compared 0.5 % short;
  - each node has a shadow-bias level v: 3 for terrain blocks and tile-grid
    floors, 1 for other grid set pieces, 0 otherwise. For v > 0 the
    original divides receiver distances by k_r = 1 − 0.005·v and caster
    distances by k_c = k_r − 0.01, so a surface keeps about 1.5 % of the
    light distance between itself and its own shadow. That is what keeps
    floors free of acne at grazing light. Exarust carries v in the mesh
    tag and applies the receiver side as that self margin (0.995·k_c/k_r);
    the cross terms between levels are left out until the cube maps are
    Exarust's own;
  - in a supersampled frame each sub-frame takes two of the eight taps
    (sub-frame i takes taps 2i and 2i + 1) and the turn is per window
    pixel, so each output pixel still averages all eight. Exarust's
    sub-frames are the four pixels of each 2 × 2 block of the scene image.
    The GI lookup splits its taps the same way.
- The sun's colour per unit of intensity is 160 lux in Exarust's scene units
  (scaled back in the shader).

## Fires

A model node with a fire emitter (`0x3DD0E002`, size s; a torch's `_Fire`
is 0.2) becomes an omni light:
- colour sRGB (1, 0.68, 0.43);
- intensity 8√s and radius 1500√s cm, ×1.25 for 0.2 < s ≤ 0.35, compared
  in double precision: a torch's stored 0.2 (0.2000000030) is above 0.2, so
  a torch's radius is 838.5 cm (reaching zero at 1048 cm); larger fires use
  other templates;
- placed 5 cm above the node, casting shadows.

Every tick the intensity is density · (1 + 0.05·x) · base:
- x is a soft-bounded random walk. With u uniform in [0, 1): v −= u past
  +1, v += u past −1, otherwise v += 2(u − ½); then x += 0.3·v·u.
- The density ramps in as 0.8·density + 0.2·(live particles / maximum).

Candles have no fire and give no light.

**Flame particles** (`exa_world::flame`, small fires such as torches; all
lengths in cm, and the simulation runs at the 83 Hz tick):
- **Template** for size s:
  - rate round(250s) per second, lifetime 1.25 s (103 ticks);
  - half-size 10·(0.5 + 0.5s), growing by twice that over the life;
  - spawn ring 35·(0.5s + s²) (4.9 for a torch);
  - buoyancy −1.25s × gravity (it rises);
  - pull toward the axis 0.0007 per tick; reach 140s; "full" count
    round(450·(0.9s² + 0.1s)).
- **Emission**: an accumulator drops by 1 with probability ⅔ each tick
  (and faster after several deaths at once). While it is below 1, a
  particle spawns and the accumulator gains (83/rate)·⅔. New particles
  start at rest at r·(cos θ, 0, −sin θ) from the node, with r = ring·(1 −
  u⁴), inheriting 0.3 of the node's own motion.
- **Each tick**:
  - a particle loses an extra tick half the time;
  - its life is cut to round((reach − D)·L/reach) when that is shorter,
    with D = |(x, 0.7y, z)| from the node;
  - dead particles at the head are dropped;
  - Verlet step: d = p − q, d.y += gravity·buoyancy, then the pull
    toward the axis.
  
  A torch's wisps live about half a second and rise about 40 cm.
- **Drawing**: each particle is a diamond facing the camera, with its long
  axis along its motion (plus 0.25 cm/tick up, so resting particles point
  up). Half-width = half-size + ℓ·t²·growth, where ℓ is the sine between
  the motion and the view. It is stretched along the motion by
  (2 − 2/(|w| + 1))·0.6·angle.
  - UVs: one of the four wisps of `FirePrt02` (a 2 × 2 atlas), under one of
    the eight symmetries of the square.
  - Colour: eleven keys over the life (RGB × 0.7 for keys 2–10; odd ring
    slots run one key later), used as linear bytes/255.
  - Output: texture × colour, premultiplied (ONE, ONE_MINUS_SRC_ALPHA), with
    no depth writes, drawn oldest first.
- The fire's density follows 0.8·density + 0.2·live/full each tick; it
  settles a little above 1.

## Rooms and black fill

The original paints nothing black: it clears the frame to the
environment's background, (0, 0, 0) in every tile-grid level, and leaves
undrawn whatever the player should not see (`exa_world::rooms`,
`exa_world::room_view`, `exa_render::shell`).

- **Rooms.** The grid's open cells are flood-filled into rooms at load
  ([formats/RFC.md](formats/RFC.md), Grid building). Each frame the
  active rooms follow the anchor: the player, or in the level view placed
  with `--camera` the point 20 m ahead of the camera (the original's
  camera distance to its target). Active are the anchor's room and each
  room behind a door of the anchor's cell whose point, 1.5 m inside the
  cell towards the door, is within 2 m of the anchor (every door counts as
  open). Door neighbours of active rooms are adjacent; the other rooms are
  hidden. A room stays in the active list, in order, while it fades: in
  over 1000 ms when it becomes wanted, out over 1000 ms when it no longer
  is; a fade that turns around keeps its alpha. States only drop when a
  room leaves the list (its door neighbours become hidden), so a room left
  next to no active room keeps its last state, as in the original. The
  fade clock is real time [guess: the original's clock was not traced].
- **Shells.** Every placed piece's `[…` children (boxes from the floor to
  2.7 m behind the facade: most reach the cell edge, as `gews_swa01`'s
  fill the 55 cm behind its 20 cm facades, but the plain walls and inner
  corners of `gews_tsa01`, `gews_uwa01` and `gews_UwA01p` stop 2.6 m from
  the cell centre, 40 cm short of the edge) are merged per room into an
  occluder; its `+R…` children and the sets' ceiling slot pieces into a
  lid (flat at 4 m, ramping to 2.64 m at the cell edge). The original writes them into depth before anything else,
  colour writes off, so what lies under them keeps the black clear colour.
  Exarust draws them as opaque, unlit pure black: the same image over a
  black clear, without depending on draw order. They cast no shadows,
  receive no light, fill no GI and have no collision.
- **What each state draws.** Hidden: nothing of the room (its meshes are
  hidden; its lights, GI emitters, shadow links and colliders stay).
  Adjacent: the room with its occluder and its lid, so from above it is a
  black block with the lid's silhouette. Active: the room and its
  occluder, so every wall hollow is black with a crisp top. Content in no
  room is always drawn.
- **Fades.** A room fading in or out has its lid drawn last, after the
  particles, black at alpha 1 − (now − t)/1000 ms entering (the fade ends
  below 0.01) or (now − t)/1000 ms leaving, blended (SRC_ALPHA,
  ONE_MINUS_SRC_ALPHA) without depth test or depth write.
- **Rooms of the level's content.** Grid pieces and terrain blocks take
  their cell's room; props under the level root, placed objects and each
  top node of the level's own model take the room of their meshes' world
  bounds' centre; props under props and meshes added later (flames) take
  their root's room; characters are hidden by the room of their position.
- **Debug.** `--all-rooms` and the level overview (no `--camera`) show
  every room active, without lids or fades.

## Cameras

- Vertical field of view in degrees.
- Main menu: 60°, eye (0, 5, 7) m looking at (0, 0, −2) m.
- Character creation: 50°, looking at (0, 0.6, 0) m from 4 m along the
  direction of (−2, 2.1, 3.6) m.
- Play: 45°·√(1.6/aspect) at the reference zoom, clamped to 10°–100°.

## Character views

`exarust character` shows the body in the original's character-creation
scene:
- the floor scene `sgccflr01.rfc`: a cobbled plane and two omni lights, a
  white key light (intensity 3, radius 14.6 m) at (1.43, 2.43, 1.50) m and a
  warm one, sRGB (1.0, 0.73, 0.4) (intensity 3, radius 8.3 m), at
  (−1.39, 1.53, −1.81) m;
- no sun. The screen's grey ambient (sRGB 0.3) only scales the voxel GI,
  and nothing in the scene fills the GI volume, so no ambient light reaches
  the body or the floor;
- the environment's second fog mode: every lit surface fades to black with
  its horizontal distance d from the origin, by a·(2 − a) with
  a = min(d / 2 m, 1), over a black background;
- the camera above.

The inventory and trade dolls light the body differently (a sun of sRGB
0.75 grey × 3, ambient 0.5, no floor); they are not shown yet.

## Not done yet

The emissive scale of a fire's parent mesh by the fire, the emissive
map's alpha on unlit surfaces without a fall-off, the jittered
supersampling pattern, medium and large fire templates, the sun and
terrain in the GI volume, several resident GI blocks, distance fog (the
first fog mode), vertex dirt (dirt modes 1–3), glass refraction, effect programs (smoke,
powers), the hair program, terrain parallax and wetness, and the outdoor
environment cube. Rooms: adjacent rooms are not clipped to their door
portals (an oblique view can show a sliver of one past its lid's edge, and
the 40 cm band between a short occluder and the cell edge can show from a
hidden room's side; in a terrain-floored cell of `exanima01` the band
shows ground when seen from above a neighbour that draws no lid, such as
solid rock [whether the original shows it too is not checked]), the lid
is not drawn into the lights' cube faces, light does not pass through the
rooms' portals (the wall flood of [Lights](#lights) bounds it), and doors
have no open state.
Software rendering here cannot afford the light shadows, so test frames use
`--no-shadows`.
