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
- The original keeps up to nine blocks resident around a point 20 m in
  front of the camera. Exarust keeps one block centred on that point, and
  rebuilds it (with a new warm-up) when the point moves more than two
  sectors. The level's points and floor triangles are collected once in
  world space.
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
- each light adds lin(colour)·intensity·multiplier·0.15·0.01 · (1 − d/R)⁴
  · open · shaping;
- the lights injected are those whose reach overlaps the block, nearest its
  centre first, up to 32. The original injects each sector group's lights,
  up to 12 per group.

Light builds up toward about direct/(1 − k). With open walls and dark
sectors around it, the menu room converges within a few hundred steps.

**Open question:** with every input verified against the executable, the
menu still renders darker than the original's capture. The floor's bounce
light is about 0.005 linear, and the torch-lit walls are about 1.45× darker
even without GI, so part of the gap lies outside the GI. This needs a
side-by-side check on the GPU at the same camera.

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
- Shadow maps cut alpha-tested surfaces (cobwebs, grates) the way their
  colour is cut. Surfaces with opacity ≤ 0.8 (glass, decals) and those whose
  record has extra flag `0x4` cast no shadows.
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
    16 bits, with near plane 3 cm and back faces culled (two-sided
    materials cast both sides). Alpha-cut materials cut their shadow as they
    cut their colour;
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

## Cameras

- Vertical field of view in degrees.
- Main menu: 60°, eye (0, 5, 7) m looking at (0, 0, −2) m.
- Character creation: 50°.
- Play: 45°·√(1.6/aspect) at the reference zoom, clamped to 10°–100°.

## Not done yet

The jittered supersampling pattern, medium and large fire templates, the sun
and terrain in the GI volume, several resident GI blocks, fog, vertex dirt, glass refraction, effect programs (smoke, powers), hair and skin
programs, terrain parallax and wetness, and the outdoor environment cube.
Software rendering here cannot afford the light shadows, so test frames use
`--no-shadows`.
