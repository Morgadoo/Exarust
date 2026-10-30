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
  packed specular) as linear. The `.rfi` format word decides: an
  uncompressed word is data when bit 16 is set (`0x0111`, `0x1111`), a
  compressed one when its top byte is `0x82` or `0x92`; the rest is colour. A loader can force
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

The level table gives each level its sub-view count and first jitter
entry: levels 0–4 draw 1, 2, 3, 4 and 8 sub-views (levels 5–7 of the table,
15, 24 and 66, are beyond the settings' range), up to four a pass, one per
quadrant. Each sub-view's frustum edges move by `2·half-extent·jitter /
size` (single precision, `size` the render size on that axis). The render
size is the window's: the view's separate render-scale mode, which would
draw the sub-views at `3·trunc(w/4)`, `2·trunc(w/3)`, `3·trunc(w/5)` or
`trunc(w/2)` and offset the resolve by 0, −1/3, −0.2 or −0.5, starts at 0
and is not changed in play. The shared buffer is twice the render size.
The final pass's sharpen is 1.5 × (0.075·level + 0.4) in double precision.
Each pass carries its own state: the first pass of a level of two to four
sub-views draws in state 3, the first of a wider level in state 1 (the
slider never reaches one), later passes in state 2 and a single view in
state 0; the low bit raises a renderer flag and the particle pass swaps in
the visibility mask `0x6000_0000` for state 0 and `0x2000_0000` otherwise.
Every pass draws `min(count, 4)` sub-views; the pass's shared jitter is the
level's entry `first jitter + pass` (not `+ 4·pass`). The resolve reads the
level's jitter table from entry `first jitter + 4·pass` with the tap
fraction `(4·pass + 1) / count` (a double division stored in single) and
is drawn with the weight `1 / (4·pass + 1)` in single precision; the projection
takes a depth dither of `0.5 − (4·pass + 0.5) / count` in single precision,
and each sub-view's animation clock is `i·step + 4·pass·step + step/2 −
duration/2` with `step = duration / count`. The resolve replaces on the
first pass and adds on later ones.
The view's projection takes half the field of view in single precision,
divides the near distance by cos/sin of it (the cosine in double precision,
the sine in single) and multiplies the half height by the aspect in single
precision. `exa_render::view` holds the table, the jitters, the render
scale, the projection, the point-to-window projection, the pass schedule,
the framebuffer viewport and region stacks and the view copy's quad, pinned
by tests.

The original's per-sub-frame projection, view and object matrices and their
viewport shifts, animation clocks and depth-lane offsets are
`exa_render::subframe`, built from the view's tables.

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

The two fade controls (`post::FadeControl`) start at the GUI clock. The
screen fade runs 5000 ticks: its alpha is `1 − (start + 5000 − now)/5000`
(0 at the start, 1 at the end) in double precision, its desaturation is
the base times that alpha, and its sharpen squares the alpha and raises it
to the tenth. The desaturation fade runs 3000 ticks: its ramp is
`1 − (start + 3000 − now)/3000`; below a quarter the mask is `1 − 4·ramp`,
else `(ramp − 0.25)·1.333`, inverted at the end, and a non-zero kind adds
`(1 − mask)²·0.4` to the bloom. Exarust exposes the values; nothing
animates them yet.

The ACES curve found among the shader programs is unused.

## Shader builder

The original does not ship fixed GLSL binaries: `Resource.rpk`'s
`shading.flb` holds templates, and `TShaderBuilder` expands one per
permutation at run time (`docs/formats/FLB.md`). Exarust generates the same
text, for comparison, in `exa_render::shader_template` and
`exa_render::shader_program`, and prints a material's program with
`exa-tool shader <library> <material>`.

**Programs.** The material factory turns a record into a material word and
a shader word whose low nibble picks `shading_f` (0 legacy, 2 unlit),
`shadeopt_f` (1 PBR), `shadehar_f` (3 hair), `shadesss_f` (5/6 skin) or
`shadetrl_f` (7 translucent); every set shares the vertex program
`shading_v`. A non-zero subsurface colour (the record's secondary colour,
RGB) lifts it to skin at shading quality 3 and above, and the translucent
mode (flag bits `0x300` = `0x100`, type 1) overrides to 7. Every map bit
needs both the record's flag (passed by its type's plug-in) and the layer;
the rules are in `docs/formats/FLB.md`.

**Flags and numbers.** The templates test the `Mat.`, `Shd.`, `Obj.` and
`Env.` words, and the integers `Lights` (0–6), `SdwSmps`, `ShadeQual` and
`DspSdws`. `SdwSmps` is the shadow quality's `2^(quality − 1)` taps split
over the supersampling passes: at shadow quality 4 and 4× supersampling each
sub-frame takes 2 of the 8 taps (so an output pixel averages 8); without
supersampling it takes all 8. `ShadeQual` is `quality − 3` from quality 4,
in shader-word bits 20–21, and `DspSdws` is 0 without displacement shadows,
else 4 over the super-sampling factor, at least 1 (1 at the 4× setting).
The super-sampling setting's table has eight entries, 1, 2, 3, 4, 8, 15, 24
and 66 samples; a quarter draws `min(samples, 4)` sub-frames (three at the
3-sample setting) and a frame `ceil(samples/4)` quarters, whose sub-frames
share `0.5 − (4q + 0.5)/N` as `Global`'s frame offset.

**Permutations.** For a material the builder compiles one key per light
count 0–6 for lit surfaces (none for unlit), keyed by the object and
environment words as well: vertex colours, deformed vertices, quad rendering
(supersampling and `PassIdx`), GI, fog mode, outdoor, wet floor, tone map
and sharpen. The game warms 29 common material/flag rows and their light
counts (`exa-tool shader --warmup`). The builder also has a separate
shadow-declaration callback, used by the voxel-GI manager's own builder:
it declares one `Shadow<i>` per light slot and tests every slot's kind
(`interface_shadow_samplers`), while the fragment interface above tests
only the first slot and declares every later one as a cube. The `SdwSmp`
table is eight three-float taps (`shader_bindings::SHADOW_TAPS`); the sun
shadow map reads a tap's first two lanes.

**Uniforms.** `Global` is 252 bytes, `Material` 112 bytes and each light
three `vec4`s; the fields and their values are in `docs/formats/FLB.md`.
`ViewPos.w` and `TimeDlt.x` are the same value: the quarter's time offset
(0 outside super-sampling, `0.5 − (4q + 0.5)/N` for quarter `q` of `N`
samples) times the frame scale (1 when the last frame took at most 1/60 s,
else 16.666… ms over the frame's time); `TimeDlt.y` is minus the frame scale
over the setting's sample count. The exposure enters neither. `FogPos`
is the midpoint of the camera's eye and the sun camera's eye, built as
`a + (b − a)·0.5`; its `w` is the scene's integer height limit (340 cm
when the scene is created) and a `y` above that limit is lowered to it.
The material writer's dirt block is written only by a dirt mode: modes 1–3 store `2 − strength` in `MatDrt.w` and the diffuse
texture's aspect in `DrtUVs`, blood (4) copies the material record's dirt
tint and skips `DrtUVs`, and a material with no dirt mode writes neither.
A spawned weapon's record tint is `(1.0, 0.06, 0.04, 0.3)`, the hair
record's `(0.45, 0.03, 0.02)`; Exarust's record reader carries no tint, so
the uniform writer takes the caller's. The block belongs to the drawing
state, so a skipped field keeps the previous draw's value; Exarust writes
zero there. An unlit record's `MatDif.rgb` is zero. A record in the
translucent or refracting mode keeps its emissive unexposed. `MatDsp` and
`DspSdw` are written only when the height map and displacement shadows
exist; the texel counts, displacement step and bias are widened to double,
scaled and narrowed, while the tap offset `1 + 0.125·PassIdx` stays in
single precision. A liquid record binds a refraction background: the
built-in `BkgRfr01` up to an opacity of 0.85 and `BkgRfr02` above it (the
compare widens the opacity to double; a NaN takes the low branch), or, when
the scene asks for its own refraction, that texture's low and high fields; a
program with local reflections binds the scene texture's low field.

**Draw state.** A draw reads the drawing state's material key (taken from
the material; its lit bits gather lights) and three words of the render
object: its own word (`0x20000` picks the object's own emissive scale over
the geometry record's), its flag word
(`0x10` the sub-frame animation clock, `0x8000` the vertex-delta stream,
`0x800000` the specular GI lookup) and its frame word (`0x20000000` the
fade, `0x40000000` half passes). It rebuilds its object word from the
factory's word plus a vertex-colour stream and the vertex-delta bit. It
gathers lights only when the object has a list and the material has the
diffuse or specular bit; otherwise it clears the six slots. Its program is
keyed on that object and environment pair; binding another program raises
the drawing state's one stale flag, which a later lit draw consumes even
when the changing draw had no lights. The material block is rewritten when
the frame's fade bit is set and `1 − fade` differs from the stored diffuse
alpha by more than `0x33FFFFD7` (an unordered compare does not rewrite),
and when the geometry asks for a per-draw emissive scale (the object's own
when its own word has `0x20000`, else the geometry record's). On the
super-sampled path the liquid level is the liquid point's world height and
the GI corner is uploaded for the environment's GI bit. A surface draws
the scene's super-sampling sub-frames (halved under sun mode 3 for the
object's `0x40000000` frame bit; the view gives its first quarter mode 3
at 2–4 samples, 1 above that and 0 at one sample, and later quarters 2),
each with its own `SdwSmp` window,
viewport shift, animation clock and, for a geometry whose flag word has
`0x20`, a depth-lane offset
`((n + 0.5)·shutter/N + lane) − shutter·0.5` in single precision (`n` the
frame's sample index, `N` the setting's sample count, `shutter` the
geometry's length), after which the sub-frame composite is rebuilt
(`exa_render::subframe`). With super-sampling off the draw takes the
single-pass variant (the one the terrain and grass draws share): one
matrix, the taps uploaded once and no sub-frame clock. Effect draws take
their parent's world matrix when they have none of their own and the
material word lacks `0x20`. `exa_render::draw_state` holds these rules;
Bevy owns the real draws.

**Program cache.** A draw interns its material and shader word pair (one
64-bit key) as a cache bucket, then asks the bucket for its object and
environment word pair (another 64-bit key); a new pair is
compiled from `shading_v` plus `<set>_f` and stored even when the build
fails (the draw then skips that program and does not retry). The fragment
entry is only built for a set name shorter than 15 characters. The pair is
the key, so the light count (in the environment word's two bits per slot)
and the light kinds are part of it. Rebuilding the libraries frees every
program and empties the buckets. The warm-up walks its 29 rows and each
lit row's seven light counts through the same cache; a row that repeats
an earlier row's material word under another shader word gets its own
bucket and programs. `exa_render::shader_cache` holds the rules, and
`exa-tool shader --warmup` prints the table and the resulting cache.

**Bindings.** `Global` sits on uniform block 0 and `Material` on block 1.
The map units pack from 0 in declaration order (diffuse or, when the
colour-slot word is exactly the opacity bit, the opacity map; then
emissive, specular, normal, gloss, height, dirt), except that the
environment cube takes unit 5, the GI volume 6, the screen copy 7 (a
record with the specular bit, which the engine's refraction path binds)
and the shadow maps 8 up. The height map's unit and the dirt map's can coincide
with the environment and GI units, as the original's do. The engine caches
`ObjMat`, `LqdLvl` (blend word `0x200`), `Light`, `SdwSmp` (a surface with
the diffuse or specular bit, or the GI volume), `VXGIPos` (the GI volume)
and `PassIdx` (quadrant rendering). A super-sampled program uploads four
taps per output pixel from the pass's own window, a plain one the frame's
taps, and a program with no tap uploads one entry (four in quadrant mode).
`exa_render::shader_bindings` holds the table; Bevy owns the real
bindings.

**Texture programs.** `textures.flb`'s entries build on their own, as one
fragment shader (no `shading_v`) linked with one of the engine's six
built-in vertex shaders (`vPos`; `vPos`+`vTex`; `vPos`+`vCol`;
`vPos`+`vCol`+`vTex`; `vPos`+`vTex`+`vTx2`; and all four), picked by the
draw state's stream word (`0x20` the second texture coordinate, `0x4` the
colour, `0x10` the first), and get the fixed `MPrj` matrix
`(2x − 1, 2y − 1, −0.001, 1)` for a texture coordinate in [0, 1]. The
linker (the same one every surface and effect program uses) binds `vPos`
to 0, `vTex` to 1, `vNrm` to 2, `vCol` to 3, `vTx2` to 4, `vTng` to 6 and
`vDlt` to 7 when the stream word has them.
`exa_render::texture_program` holds those rules; Exarust bakes the
procedural textures on the CPU instead (`exa_assets`).

**Light slots.** Each draw has six light slots. It gathers up to six of the
object's linked lights (fewer when the view's light cap is lower) in list
order, stopping at a link with no id or a
weight below 0.004 (a NaN weight passes), and orders them with the lights
whose node shadow bits (`0x300`) are clear first and the shadowed ones
after; a light keeps its slot from the previous frame where it can and the
rest fill the first empty slot of their group. A light new to its slot
writes all three `vec4`s: the position is the light's vector for its kind,
the colour is its record's linear colour times
its intensity, then `2/π / SdwSmps` (divided in double and narrowed),
then `(1 − dimmer)·exposure` in single precision; `w` is
`0.8/radius`; the shape is `(softness + 1)·(quality·0.005)` with the step
in double and narrowed (`0.003` under `FastLgt`), and for a sun-like light
`softness·0.5` in single precision times the double `0.005`, narrowed and the vertical
shaping `(y, z, w)` when the node's shadow bits are exactly `0x200`, else
`(1, 1, 1)`; the position's `w` is `(0.97 + E)/(radius · scene
light scale)`, where `E` is a light-record field distinct from the copied
softness the radius uses [its meaning unconfirmed], or the sun's own depth
scale. A light that stays in its slot
only has a shaped light's radius refreshed (which does not mark the light
records stale) and, when the scene's light scale changes, its depth scale.
The environment word carries `0x200` for a light with no shadow bits and
`0x100` for a shadowed one, two bits per slot. `exa_render::light_slots`
holds the fill; Exarust's WESL reads Bevy's clustered lights instead, and
`exa_world::shadow_links` already chooses a mesh's six lights.

**Fixes from the decoded programs.** Comparing every original program with
Exarust's WESL equivalent corrected these (all are the template's maths, not
a guess):

- PBR F0 without a diffuse map mixes the other way:
  `mix(MatDif·MatSpc, 0.04, metal)` (the `shadeopt` template's `{?ELSE?}`
  branch), not `mix(0.04, albedo·MatSpc, metal)`.
- An unlit surface with an emissive map and no fall-off takes the alpha
  MatEmi.a × the emissive map's alpha and never samples an opacity map;
  the fall-off path takes MatEmi.a + the fall-off and ignores the map's
  alpha.
- A two-sided back face flips only the normal map's `z`, not the tangent
  frame: `nrm = t·x + b·y + (−N)·z` with the original's `t`/`b` from the
  unflipped geometric normal.
- The skin program's final alpha is the material opacity, not the diffuse
  map's alpha.
- Parallax's ambient darkening applies with displacement shadows too, and
  only in the PBR (and translucent) program: the legacy `shading_f` adds
  its ambient undarkened.
- The GI lookup's disc frame is the original's
  `(1 − n.x²a, −n.x n.y a, −n.x)` / `(−n.x n.y a, 1 − n.y²a, −n.y)` with
  `a = 1/max(1 + n.z, 0.1)`, not an arbitrary frame.
- Inside the GI volume only a record whose reflection layer the factory
  keeps (`Mat.TexRef`) reflects the cube; the rest use a quarter of the
  ambient (above). Outside the volume every surface reflects the cube,
  layer or not (the non-GI branch has no `TexRef` test). Before, the
  loader flagged every material with the cube bound, so indoor stone and
  wood reflected the cube; now only metals, glass, eyes and skin (the 30
  records with a reflection layer in the shipped libraries) do. The cube
  stays bound for the ambient outside the volume.
- A legacy record's diffuse colour is multiplied by 1.5 before the sRGB
  decode, so 0.5 becomes linear(0.75) = 0.522, not 1.5 × linear(0.5) =
  0.321.
- The glossiness is raised to 0.01 only for type-0 records (the loader's
  rule, with no upper clamp). The specular ×3 with a specular map is the
  material writer's: programs whose class has bit 0 set (PBR, hair, a skin
  lifted by a scattering colour, translucent), not legacy (0) or type-4
  skin (6).
- The parallax texel scale rounds to nearest, ties to even (the
  conversion instructions' default mode), in the writer's order:
  `P = round(50·amount)`, `Y = round(P·(2048/height))`,
  `X = round((height/width)·Y)`.
- The PBR `Lighting` clamps `N·H` to [0, 1].

**Programs drawn at run time.** The material loader turns the factory's
program choice into shading flags that `exa.wesl` branches on (one
pipeline, a uniform branch per material, so no extra draws or pipelines):

- **Hair** (class 3, `shadehar_f`; `flags::HAIR`, `hair.wesl`,
  `exa_render::hair_program`). The frame is the program's own: tangent and
  bitangent from the unflipped vertex normal, that normal flipped on every
  back face whether or not the record is two-sided, the normal map's `xy`
  with `z = √(1 − min(xy·xy, 1))`, no normalisation; the strand frame is the
  tangent re-orthogonalised against the shaded normal. Each light (colour
  `C`, attenuation and shadow included) adds a specular
  `F·C·gh·d·π·(0.1·D(0.8, 0.4r², b·h + 0.05) + 0.3·D(0.8, 0.6r², b·h − 0.05)·MatSpc)`
  with `D(rx, ry, y) = 1/(π·rx·ry·(cos² + sin²·((t·h/rx)² + (y/ry)²))²)`, `cos = n·h`,
  `F = 0.05 + 0.4·2^((−5.55473·h·v − 6.98316)·h·v)`, `gh = N·L/(N·L(1 − k) + k)`,
  `k = (r + 1)²/8`, `d = 0.25/(N·V(1 − k) + k)` and `r` the roughness; and a
  diffuse `(1 − F)·C·(0.4·N·L·MatDif + RimCol·((1.15 − |v·h|)²·max(N·L + 0.75, 0))²)`
  with `RimCol = normalize(MatDif)·0.6 + 0.4`, so a light behind the strands
  still rims them. The ambient adds `amb·(1.5 − 0.75·v·n)·MatDif`. The
  diffuse map's colour multiplies the sum only under dirt mode 4 (with the
  blood factor), as `MatDif·map·(1 − metal)` with a specular map and the map
  alone without one; the shipped hair record (`harhps1`: type 3, material
  flags `0x720007`, extra `0x100`, no dirt mode) therefore shows its picked
  `MatDif` colour, and its diffuse map only shapes the strands' alpha,
  `(a − 1 + MatEmi.a)/MatEmi.a` (MatEmi.a = 1), times `MatDif.a` once more
  at the end (an opacity map alone gives `MatDif.a·a`, not squared). With
  `MatSpc`, the reflection is the cube at LOD `(r − 0.05)·8` about the
  normal bent 70 % toward `(−v × b) × b`, times
  `amb·(1 + MatSpc)·Ambient.w·0.5` and weighted
  `0.1 + 0.5·2^((−5.55473·N·V − 6.98316)·N·V)`; a specular map's red then
  scales the whole specular. The emissive adds unweighted. Two guesses are
  labelled in the shader: a hair mesh without tangents builds its strands
  across the world's up axis, and a record with `MatSpc` but no reflection
  layer (whose original program does not compile: its reflection is
  undeclared) is drawn without the reflection. Not reproduced: the
  `Wetness` terms (`Gloss = mix(Gloss, Gloss², W)`, the colour
  `× (1 − 0.3·W)`: the Global value is the scene's wetness, which Exarust
  does not model) and the outdoor branch's `vgi` in place of `amb`. Seen
  on every character with hair (the arena fighters in `arenao3`/`arena1`
  and any character in exanima01, with `--screenshot`): the strands take
  a flat picked colour with a bright anisotropic band and a soft rim
  instead of the textured PBR look.
- **No `MatSpc`** (`flags::NO_SPECULAR`): every program adds its specular
  sum (lights and reflection) only for a record whose material word has
  `0x4`. A lit record without it but with a reflection layer, or with a
  packed specular map, had shone in Exarust; it no longer does.

Not reproduced from these programs: the legacy unlit program's vertex-colour
branches (`vrCol × EmiTex × MatDif.a`, and `vrCol × OpcTex.a`), its
`BlendMode = 4` multiply branch, the translucent program (`shadetrl_f`: no
shipped library record selects it), and the
`Shd.DispSdwF` pre-pass. The legacy lit program never samples an opacity
map, so the original's alpha-tested cobweb cutout is unresolved; Exarust
keeps the opacity map in the alpha for Bevy's alpha test (both shipped
alpha-tested records have opacity 1).

## Pass order

The original renders a frame in one fixed walk over its per-material
render-state lists (`exa_render::passes` holds the model and the two list
orderings, pinned by tests):

1. the environment's per-frame setup (fog, background colour) and the
   clear; `exa_world::environment::live_environment` runs that setup once a
   frame for the loaded level (the weather tick, then the frame's fog,
   clear, ambient and sun light, published only when a value changes); the
   environment's sky hooks around the lists are empty;
2. the alpha-tested class (`0x8000`), then the opaque class, both with
   depth writes on;
3. the after-opaque hook list;
4. two more hook lists with depth writes on, then the depth test on and
   depth writes off;
5. the `0x10000` class (decals) with every view's projection `[2][2]` term
   raised by 2·10⁻⁵, a depth bias that lasts through the blended lists;
6. the region scissor off, then a hook list (the frame copy refracting
   surfaces read is reached through a hook, by the earlier notes; the walk
   itself does not copy);
7. the blended class, one list for a sectorless scene or one per scene
   sector, still biased; the bias is removed after them;
8. a hook list, then the particles: a depth-only pre-pass (colour writes
   off, depth writes on) and their colour (depth writes off);
9. the environment's weather effects, the closing hook list, then depth
   writes back on.

A render state's blend layer is bits 4–7 of its blend word
(`passes::blend_layer`); the low four bits are blend factors and play no
part in the order.

The draw of a list (`passes::plan_list`) walks its material groups in list
order, binds a group's program once, and draws the group's entries whose
flag word shares a bit with the renderer's visibility mask; a group with no
visible entry is not bound at all. Sectors are walked in scene order under
the same test (`passes::visible_sectors`); a sector with its own rectangle
sets it as the scissor region and turns the scissor test off before its
lists are drawn. Within a list the test is switched on for visible entries
whose word has bit `0x1000_0000` and off for those without it, only when
the bit changes from the last visible entry, starting from off
(`passes::scissor_switches`). A blended entry binds the group's full state
for itself, with the group's alpha at `1 − opacity` and the blend word
`0x203`, then puts back the saved alpha and `0x100`
(`passes::entry_alpha`, `passes::ENTRY_BLEND_WORD`).

The particles own two lists (`passes::ParticleList`; the second's groups
bind a texture in the depth pass, so it is called the textured one). Their
pass swaps the renderer's visibility mask for `0x6000_0000` (single view)
or `0x2000_0000` (super-sampled) and walks the scene's particle sectors
(flag word bit 0) twice over: first depth-only with the colour writes
masked, the first list for all sectors untextured and then the second for
all sectors with each group's texture bound at its first visible entry;
then, with the colour writes back on and depth writes off, per sector the
first list and the second (with one shared program-state bit, `0x80`,
cleared) with the per-entry `1 − opacity` alpha. The old mask is restored
after the pass (`passes::particle_pass`).

The opaque list is grouped by material program key: the first state stays
first, each next slot takes the first unused state with the current key,
and when none is left it takes the last unused state and moves to its key.
This is a grouping, not an ascending sort. The blended list is selected by
the lowest blend layer (bits 4–7 of the blend word) first and then the
highest opacity; the opacity to beat starts at −1 for each slot and is
reset neither by a lower layer nor by the first state found by layer
alone, so an opacity seen earlier in the scan still has to be beaten.

`passes::state_list` is the factory's list choice from a material's flag
word: `0x10000` first (the decal list), then `0x8000` (the alpha-tested
list), then the low nibble (0 opaque, 1–4 blended, anything else no
state); the second renderer mode sends every class but a blending nibble
to the decal list and files a blending nibble in a list of its own.
`passes::existing_state_list` is the renderer's lookup of an existing state,
which ignores the `0x10000` bit and the mode, and `passes::released_flags`
is the word a released state keeps (detail levels below 3 and 4 clear bits
21 and 23).

`queue::DrawQueue` keeps the deferred draws: at most 1023 entries, each in
the bucket of its sector's light count (the chain's enabled lights whose
weight, widened to double, is not below 0.004, capped at the renderer's
count and six; a NaN weight counts). The queue is a list kept grouped by
bucket: a draw whose bucket is the one the lights were last set up for goes
to the head (once, until the next flush); any other goes right behind the
first entry of its bucket that has a successor, or at the end, so a group
reads first-entry then newest first. Appending to a full queue flushes it,
and a flush remembers its last entry's bucket for the lights.

Exarust lets Bevy's phases take the place of the fixed walk: `Opaque3d` for
the opaque class, `AlphaMask3d` for the alpha-tested one and `Transparent3d`
for the depth-write-free classes and the particles. The clear is the
camera's clear colour; the decal bias, the particles' depth pre-pass and
the hook lists have no separate node yet. `exa_cube_shadows` and `exa_gi`
run in the `Core3d` prepass, and the post chain follows the transparent
phase. The original's list order inside a phase is not reproduced by Bevy's
batching (the same materials and meshes batch in any order); draw order
between different materials follows the phase's own sort.

## Light nodes

A level's light node reads its flags, colour, multiplier, intensity, radius
and softness; `exa_world::light_bundle` turns them into the Bevy light, the
GI share and the shadow reach (`read_light`, `shadow_radius`,
`vertical_shaping`). A body whose flags carry `0x10000` or `0x20000` also
stores, after one skipped word, the flicker amplitude, speed and emissive
share (`exa_formats::rfc::LightAnimation`); others keep the constructor's
0.1, 0.5 and 0.5. `exa_world::light_source` runs what the original does
with them:

- **activation** copies the base intensity into the current one and picks
  the shadow-quality offset from the low two flag bits: 00 → 0, 01 → +1,
  10 → −1, 11 → −2 (not applied: the shipped cube pool has one size);
- **a lever** sets the scale: `intensity = base × scale`, and the parent
  mesh's emissive scale becomes `scale × share`;
- **flicker** (flag `0x10000`), once a frame with the step
  `speed × frame ms × 0.01` (the frame time held to 0…16.667 ms): the term
  drifts down by `u·step` at a scale of 0.9 and above, up by `u·step` at
  −0.9 and below, and by `2·step·(u − ½)` between (the bounds compared in
  double precision); then `scale += frame ms × 0.1 × u′ × term` in double
  precision, held to ±1. The intensity is `(scale·amplitude·0.1 + 1)·base`
  and the parent mesh's emissive scale `scale × share + 1`. `u` and `u′`
  are two draws of the engine's `× 0x08088405 + 1` generator, in that
  order;
- **fade** (flag `0x20000`) draws nothing: with a positive term the scale
  rises by the step and settles at the base intensity on reaching one,
  otherwise it falls and settles at `amplitude × base` on reaching the
  amplitude; settling clears `0x10000`. The parent mesh's emissive scale is
  `scale × share`.

The parent mesh's emissive scale goes to its first part (the original
writes it into that sub-object; which uniform its draw scales is not
traced, so Exarust scales the part's emissive colour, on a material copy
of the part's own). A light at rest leaves its mesh as loaded. Per-sector
light aliases are not reproduced.

The light's derived values (`LightSource::direct_colour`, `derive`,
`node_colour`, `update`) are a tested model, not yet consumed by the
shader path. An update clamps an intensity under 2⁻²³ to zero (an ordered
compare: a NaN stays) and then:

- the direct colour is the light's colour times `√intensity · 0.75` in the
  legacy workflow, or the linearised colour times the intensity in the
  linear one;
- the second colour slot is the direct colour, or black when the light
  node's flags word (`flags2`, not the body's flicker and fade bits)
  carries `0x20000`; without that bit the derive adds it to the running
  colour sum, which the update then replaces by the direct colour and
  takes its Rec. 601 luma (`exa_formats::colour::luma`, red first);
- the GI colour is the direct colour times the record's multiplier, or
  black with node flag `0x10000`;
- the light node's own colour is the linearised colour times
  `√intensity · multiplier · 3` (linear) or `√(intensity · multiplier) · 0.6`
  (legacy, the 0.6 in double);
- the reach is the radius while the multiplier is at least 2⁻²³ (a NaN
  multiplier keeps it too, as the unordered compare does), else zero; a
  reach that moved by more than one sets pending bit 2 and every update
  sets bit 4, the shadow links' rebuild flags.

A light alias's reach (`exa_world::light_alias`) is the original's
per-node weighting, a tested model with no caller yet: the plain test is
the luma of the alias and node colours against the alias's 0.01 cutoff;
the gated test (behind the cell flood when the light's flags carry
`0x2000000`) adds the sphere overlap, the oriented-box distance under the
light radius, the falloff distance (the box distance, or the squared
sphere shell when that is larger), `max(1 − d·falloff, 0)⁴`, the spot's
vertical blend from the box's lowest point and
`max((2^(t·cone) − 1)·exponent, 0)` with the engine's fast logarithm and
exponential, the intensity gate, the colour-product luma times the
unscaled weight, and the six-face shadow mask (each face cleared when the
node lies beyond it by more than the node's bounding radius).
`SpotShape::of` builds the light's three spot constants from the record's
`(q, p)`: `1/(2^(p+1) − 1)` with the same fast functions, the flat factor
`sin(q·π/2)/2` (or `3 − 3/cos(q·π/2)` below zero) in double, and `p + 1`.
The cell flood that answers the gate is the scene-node unit's; the alias's
field copy and construction are not modelled beyond these values. The sun
map's two matrix builders are `sun_fit::light_view_matrix` (rows
`dir × up_hint`, `right × dir` and `dir`, column-major, with the negated
dot products as the translation) and `sun_fit::orthographic_matrix` (the box's `2/span`
scales and `−(max + min)/span` translation). The cube map's size comes from
the quality word through `shadow_map::cube_draw_settings` (the table
halved, the softness and the high-detail flag), and its pooled face size is
that key.

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
| `0x58` | refraction word (below 2⁻²³ in magnitude becomes 1), packed into `OpcFll.w` for refraction mode (`0x48 & 0x300 == 0x200`) |
| `0x64`, `0x68` | opacity fall-off amount and exponent |
| `0x30`, `0x34` | dirt amount, and a word the factory copies into the draw state when the plug-in-masked flags have bit 0 (reader not traced) |
| `0x7C`, `0x84`, `0x8C` | texture-coordinate translation (2 floats), scale (2) and angle; extra flag `0x40` builds the transform |
| `0x90` | render-state word: the loader keeps its bit `0x20` |
| `0x94` | plug-in mask: the factory records a record calls |
| `0x98`, `0x9C`, `0xA0`, `0xA8`, `0xB0`, `0xB4` | kept whole; no reader found |

`exa_formats::material::Material` decodes every word of the block
(`raw_block`), so a pass-through rewrite keeps it. The record loader's
order is kept: type-0 diffuse is scaled ×1.5 in stored sRGB before the
decode, the gloss floor to 0.01 applies only to type 0, and the specular
level scales the linearised specular colour.
`exa_formats::material::uv_transform` builds the texture-coordinate matrix
of a record with extra flag `0x40` in the loader's order (the engine's sine
and cosine of the negated angle, scale about the centre, then the
translation). The mesh build applies it to the group's uploaded `UV_0`
stream (`uv_transform_point`: `(u·m0 + v·m3) + m9`,
`(u·m1 + v·m4) + m10`, each sum in that order); the split tangents still
come from the stored, untransformed channel. No shipped mesh-unit record sets the flag.

A newly constructed material object holds a 3 × 3 identity texture transform
with a zero translation row (twelve floats), the texture load word `0x700F0`
(0xF0 and repeat wrapping in the `0x70000` sampler bits) and a physical
divisor of 1; the parameter record does not store the transform. The
manager's built-in `_Default` material is a full 184-byte record: mid-grey
diffuse (0.5, 0.5, 0.5, 1), a 0.22 grey specular colour, opacity 1,
glossiness 0.1, specular level 1, render class `0x100`, flags 7 (lit and
specular), kind 1, load word `0x70000`, displacement step 1 and bias 0.5,
fall-off exponent 2, and no texture layer. It is created on first need with a
count of `0x3FFFFFFF` and gains one user per request. The manager keeps its
own provider in slot 0 with the global plug-in providers behind it, a
material table that reuses the first empty slot from a search hint, a list
of the loaded slots, a list of objects waiting to be freed and the slot of
`_Default`; a get-or-load finds the name in the table or asks the providers
from the last to the first and falls back to `_Default` for an unknown name
or unreadable data. Dropping a count below zero is an error; reaching zero
queues the object for freeing and empties its slot
(`exa_assets::material_manager`, bookkeeping only, not driven by the
loader; it also answers whether a provider can give a material's data and
whether any provider knows the name, the two presence probes the original's
own provider and import pass use).
An unknown name in Exarust shades with the `_Default` record, shared by all
such names.

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
  replaces this term (see Voxel GI). Exarust uses the cube path only with
  `--no-gi`; an object no resident GI block holds takes no GI light, as the
  original's texture 0.
- The cube is `genref01r` for every indoor level (six 256² sRGB faces, each
  with nine mips); outdoors it is `genref02r` masked by `genref02rm`. The
  level record's `EnvMapIA01` is unread.
- Reflections sample the cube along the mirror direction at mip
  (rough − 0.1)·8, as (e + e²) × level ambient × 8 × the geometry term,
  for every surface without voxel GI. With it only a record with a
  reflection layer kept by the factory (`Mat.TexRef`, slot `0x100000`:
  metals, glass, eyes, skin) samples it, at mip (rough − 0.05)·8, × the GI
  ambient × 8 × 2; every other surface's specular GI there is **a quarter
  of the ambient** (the shader's `{?Mat.TexRef?}` alternate). A record with
  the layer binds its own cube (`ModelLoader::environment_map_named`, loaded
  like the scene's); every other surface keeps the level's.
- Roughness ("Gloss" in the shaders) depends on the maps present:
  - 1 − glossiness × specular map green;
  - otherwise 1 − glossiness × gloss map red;
  - otherwise 1 − glossiness.

  Glossiness is the record's `0x3C` as stored. Gloss maps are dark, so the
  menu floor's 0.9 becomes a matte roughness of about 0.8.
- The specular colour is zero unless the material word carries the
  specular bit (the colour times the record's level) or only the
  reflection layer (the unscaled colour). It is tripled only for programs
  whose class has bit 0 with a specular map.
- Dirt maps only act through vertex colours. Many prop and level meshes
  store a colour stream (859 of them draw a material with a dirt mode,
  `exa-tool corpus colours`); Exarust keeps it in `Mesh::colors` but does
  not upload it, as it shades no dirt mode but 4, wound blood
  ([Blood](#blood)), whose stream the blood system makes itself.
- Tangents are generated from the UVs, and the original's bitangent runs
  along −v (its tangent sign is the opposite of MikkTSpace's). Exarust's
  `exa_assets::tangents` reproduces the original's splitter: per-face
  tangents accumulated unweighted per vertex, a determinant floor of 1e-10
  (f64), the tangent projected onto the surface normal and normalised (a
  zero one stays zero), and the sign 1 when (n × t)·b < 0 for the
  unprojected t, else −1. The normal itself is the angle-weighted sum of
  unit face normals over the smoothing group, with the original's
  arc-cosine approximation for the corner angle,
  `√(1 − cos θ)·(1.571 − 0.1945 cos θ + 0.111 cos² θ − 0.111 cos³ θ)`;
  faces whose flags carry `0x10000` count only for a group no other face
  reaches.
- The program follows the material type:
  - 0 legacy;
  - 1 PBR;
  - 2 unlit;
  - 3 hair;
  - 4 skin (subsurface).
- Legacy materials (type 0) use the older program (their diffuse colour is
  brightened ×1.5 **before** the sRGB decode):
  - no Fresnel per light, and diffuse is N·L;
  - (lights + reflection) × specular colour × (0.5 + 0.5·Schlick);
  - the specular map multiplies the light and reflection specular at the
    end (there is no uniform ×3 for legacy materials).
- **Emissive** (MatEmi) is lin(emissive colour), doubled when the
  plug-in-masked feature word keeps `MatEmi` (`0x8`), the lit or specular
  bits and the emissive layer (torch handles, lamp bodies). It adds on
  top. With an emissive map the PBR program weights it by N·V²·z + 1 − z,
  z the fall-off's amount term (0 without a fall-off).
- **Unlit** materials (no lit or specular bits) show only their emissive:
  `MatDif.rgb` is zero and colour MatEmi × the emissive map, alpha the
  opacity (`0x38`, not the colour's alpha, and not clamped), plus the
  fall-off when they have an emissive map. With an emissive map and no
  fall-off the alpha is MatEmi.a × the map's alpha; a bare opacity map is
  only used when there is no emissive map (the two are never combined).
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
- In the PBR program parallax darkens the ambient by min(h − ½ + 1.2, 1)
  whenever a height map exists, with or without displacement shadows (the
  template's `Mat.TexDsp` branch is separate from `Shd.DispSdw`); the
  legacy program does not.

**Textures** are sampled with the original's settings at its top texture
quality (`TextureQuality = 5`) and 4× supersampling:
- anisotropic filtering 2^(4 − mode), so 16× for ordinary textures;
- a LOD bias of base · (g(supersample) · ⅓), in single precision with
  the product g · ⅓ taken first, with base −0.75 at quality 5 (−0.5 at 4, 0 below) and g = ss below
  2 and ss − 1 above. That gives −0.75. It applies
  to colour textures (diffuse, emissive, opacity, legacy specular), not to
  normal, gloss or packed specular maps; a diffuse whose record carries a
  secondary colour skips it. wgpu samplers have no bias, so the
  shader applies it (`SKIP_DIFFUSE_BIAS` and the data slots select zero).

Compressed images upload as their stored block format: BC1 and BC3 with an
sRGB variant for colour, BC5 linear for two-channel normal maps, and BC4 as
`Bc4RUnorm`, the original's single-channel luminance LATC upload. A BC4
image has one grey channel and no sRGB variant, so nothing decodes it: the
surface shader repeats its red over the whole colour (`GREY_DIFFUSE`),
which is what the 4 600 BC4 apparel and item diffuse maps show. The same
holds for a BC4 specular or emissive map (`GREY_SPECULAR`,
`GREY_EMISSIVE`): the hardware's luminance upload returned the stored grey
on every channel whatever slot bound it. Without GPU BC support every image
decodes to RGBA8 instead.

The settings screen's `TextureQuality`, `TextureDetail` and `Supersample`
reach these rules (`TextureSettings::with_settings`: the filter mask, the
anisotropy level, the bias base and the detail level). A material carries
the LOD bias it was built with, so a quality change applies to the materials
built after it; the original re-binds every texture with the new global.

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
textures. Exarust evaluates the same per-layer stacking per pixel, over
every material the tile uses (up to 16, in palette order), skipping the
layers that do not cover the pixel:
- texture coordinates (X, −Z)·f0/12 per metre;
- colour: texture × linear(tint) × 1.5·min(f9, 1), cover f9, sharpness
  f2/f3; then × 0.75 (the terrain's `MatDif`);
- normal: (N·2 − 1)·f11, mixed by the mask-sharpened opacity (f4/f5); the
  composite's z is the square root of 1 − the xy length squared, capped at
  0.999, and the vector is normalised (`terrain_nrm`). The lit program
  applies it in the frame it builds from the vertex normal s: +X projected
  onto the surface, then its cross with s (on flat ground +X and +Z, green
  along −v); the parallax's view vector uses the same frame;
- gloss: the gloss texture × f10 × 1.5·min(f9, 1), stacked like the colour
  and clamped to the 8-bit composite's range; roughness = 1 − gloss (no
  wetness indoors);
- parallax height: f20 + (B − f21)·f22 from the bump slot, stacked like the
  normals;
- parallax: scale 0.007 in 6 m block units, four accumulating steps, then
  eased back toward the unshifted coordinates by min((h₀ − h₃)·2, 1);
- Fresnel F0 = 0.04 per light; GI or cube ambient, cube reflections;
  specular × min(255·r, 1), with r the red of the diffuse colour, the
  composite already times the 0.75 (`shadetrn_f`: `difCol = DifTex ×
  MatDif`, then the mask).
- The palette's header float at offset 28 is the draw's specular level
  (`+ 0.3`, unused while the gloss pass is on) and turns on the wet-floor
  flag above 0.4; only the sewer palette ships 0.5. With the flag the lit
  program's roughness is `1 − min(G + 8c², 0.85)`, with
  `c = max((y + 35)·−0.03, 0)²·n.y` and y in world centimetres, so the
  gloss is capped at 0.85 and ground below −35 cm grows glossier. Exarust
  reads the float into `Palette::wetness`, sets the `WET_FLOOR` flag above
  0.4 and applies the term in `terrain.wesl`; `terrain::composite`'s
  `wet_floor_gloss` is the tested model of it.
- The composite's set sizes, pools, passes, program keys, per-layer values,
  quads, `TexPos` and block selection are modelled in
  `terrain::composite` (below); Exarust's per-pixel path produces the same
  layer stacking without baking the sets.

The original's composites hold about 170 texels per metre and have no mip
levels, so its terrain always shows the layers' full detail. Exarust reads
the colour and normal layers at their base level and lets supersampling
smooth the aliasing. `terrain::composite` keeps the compositor's own
numbers: a high set is 1024² (colour RGB8, normal RG8, gloss R8) with a 512²
parallax target at the default texture detail, a low set 512² with a 256²
parallax target, 21 high and 43 low sets, four layers a pass with additive
groups, the program key's textured/masked/`AddPass`/`BaseNrm` bits, the
per-layer colour/gloss/height/displacement/mask values, the block's palette
and mask quads, its `TexPos` (half the split less the block index shifted
down by the level difference, and the level's block side over the tile
side), the compositor driver's pass list (normal, colour,
gloss, displacement, four layers a group, each group's mask upload), the
mask and height passes with their clears, blending, clamping and quads, and
the selection's cut-offs, sort keys, set stealing and one-block-a-frame
rule. `exa_render::terrain_composite` hosts the set targets themselves
(colour RGBA8 stored as sRGB — wgpu has no renderable RGB8, and sRGB keeps
the 8 bits gamma-encoded like the original's — normal RG8, gloss R8 and
half-size displacement R8, at 1024²/512² from the default texture detail)
and creates a set the first time the live selector holds it, one a frame,
dropping it when the set returns to its pool. `TexPos` takes the terrain's
block level (3) and the block node's own level (also 3): the block
coordinate is half the split `2^(L−1)` less `bx` (and `by`) shifted down by
the level difference, a wrapping 32-bit difference whose shift count is
masked to five bits (so a node deeper than the block level would keep no
block bits); the third component is `2^L/4800`. The lit program adds the
tile-centred position in centimetres times it, so the set is read at 0…1
across the block
with v along +Z.

With `EXA_TERRAIN_SETS=1` the sets are also baked and drawn, as the
original shows its terrain:
- **Bake.** `terrain::composite_draw` queues one held set's bake a frame
  (in selection order) with the block's tile material: its layers, covers
  and values. The renderer runs the compositor's passes in its order —
  normal, colour, gloss, then displacement at half the side — each a
  full-target quad over the block, so target texel (i + ½)/N is the point
  (i + ½)/N across the 6 m block (`terrain_bake.wesl`). Each pass stacks
  the layers exactly as the per-pixel draw does (`terrain_layers.wesl`,
  shared by both): the normal pass writes the stacked xy as xy·½ + ½, the
  colour pass the stacked colour before `MatDif`, the gloss pass the
  stacked gloss, and the displacement pass the stacked parallax height,
  each clamped to its 8-bit target. The original's groups of four additive
  layers fold into one stack, as in the per-pixel draw. A set copy (a
  block traded from a high to a low set) binds each destination target the
  two sets share, samples the source's same target at the fullscreen uv
  and draws a full-target quad, the original's built-in copy program
  (`terrain_copy.wesl`; `terrain::composite::copy_draws` holds the pass
  list). The selector reports each copy to `terrain::composite_draw`,
  which queues it in `TerrainCopyQueue`; the blits run in the prepass,
  before that frame's bake (the block that took the high set bakes into it
  next, as in the original), so the block draws from the copied low set in
  the same frame. A block that never drew its high set, or whose bake into
  it is still in flight, is rebaked from its layers instead (a copy would
  read an unwritten target).
- **Draw.** Once a block's bake has run it takes its own material, the
  tile's with the set bound and the `COMPOSITE` flag, and `terrain.wesl`
  reads the set as the shipped `shadetrn` program does: the composite
  coordinate is the tile-local position in centimetres × `TexPos.z` +
  `TexPos.xy`; the parallax shifts it by `dsp·(h − ½)` with
  `dsp = vt.xy·0.007·(4 + 3·vt.z)` in four accumulating steps and eases
  back by min((h₀ − h₃)·2, 1), with the view taken on the lit program's own
  axes (+X projected, then its cross with the normal: the set's v runs
  along +Z); colour, gloss and normal xy (× 2 − 1) are read there and the
  lighting is unchanged. A block keeps the per-pixel draw until its bake
  has run, and goes back to it when it loses its set or the set's targets
  are rebuilt. Blocks the selection holds no set for (past 44.4 m, or
  beyond the 64 sets) keep the per-pixel draw; the original shows them only
  through a set.
- **What changes on screen.** Near the camera nothing visible: the bake is
  the per-pixel stack sampled at 170 texels a metre (high sets) or 85 (low
  sets, 15–44.4 m), so fine layer detail beyond that resolution softens,
  colour above 1 (a tint × 1.5 over a bright texel) clips at the 8-bit
  composite as in the original, and the parallax reads the half-size
  height. Checked on arenao1 with `EXA_SCENE_SEED=5`: 34 blocks draw from
  sets after about a second and the frame matches the per-pixel draw to a
  mean of about 1–3 levels in 255, the same as two runs of the default
  draw.

The full pools are about 235 MB of targets and the bake costs one block a
frame, so all of it stays opt-in; without the variable no set is created,
no bake runs and every block draws per pixel.

**Geometry.** A tile is a 241 × 241 vertex grid over 240 × 240 cells of
20 cm, and the original draws it as one terrain mesh per 6 m block: a
30 × 30 cell window of the same grid, each block in its grid cell's room.
Every block entity carries `terrain::TerrainBlock`: block (bx, by) at
`(step·bx + half − size/2, 0, step·by + half − size/2)` with
`step = size/2^level`, `half = size/2^(level+1)` and level 3, half extents
`(half, 10 cm, half)` and radius the half diagonal, the original's flat
slab bounds. Its activation gives it the floor shadow-bias level and the
room visibility is what draws it.
Every block draws one shared 30 × 30 cell triangle list, flattened into the
tile's 241-vertex rows, from its own first vertex. The list is serpentine,
cell row 0 running along +X, row 1 back along −X, and every quad splits
along the diagonal from its low corner (i, j) to its high corner
(i + 1, j + 1). Exarust builds one mesh per built block from the same patch
(`terrain::patch`), so rooms and visibility work per 6 m block rather than
per room. A hole removes the quads whose low corner
is a hole, and each tile's edge hangs a 0.5 m skirt (an Exarust addition).

**Streaming.** The original builds a tile's geometry and masks on a worker
thread and inserts finished tiles from a frame hook, at most one per frame.
Each block carries a distance rank, `round(6000 − d)` centimetres within
60 m, and the hook ranks blocks against the midpoint of the blocks already
wanted and visible, so a visible block keeps its order ahead of the rest.
Composites are chosen around the camera moved 10 m along its view and
snapped to a 6 m block centre: blocks within 44.4 m get one, those within
15 m a high-resolution set, from pools of 21 high and 43 low sets (at most
189 blocks in the list). Terrain plugins gather the 3 × 3 tiles around one,
missing tiles as an empty placeholder. The worker marks a finished tile
`(state & ~0x10) | 0x828` in the reader's state. The hook walks the tiles
with the stream bit `0x800` (nothing clears it): a ready, inserted tile
(`0xa0`) has its blocks marked wanted, and the first tile with work is
handled and ends the walk (a reload bit `0x1000` unloads it, the insert test
is `(state & 0x309f) == 0xf`, and an upload is `(state & 0x4080) ==
0x4080`); tiles with no work are passed. A refresh pass detaches a tile and
bumps the delay counter, during which frames only count down.
Exarust runs the same hook: a built tile is spawned with its nodes hidden
and the hook shows one tile a step during the load, with the
block-until-loaded wait flushing the rest before the level shows (a tile a
grid level leaves empty is built and inserted at once). `TerrainStreaming`
keeps the tiles, their blocks and the streaming bits after the load and
runs the hook and the block selection every frame (the selection is
skipped while the refresh delay counts down, as in the original):
- the frame hook handles at most one tile; an insert or a ready tile marks
  its blocks as wanting a composite, an unload hides the tile and takes its
  blocks out of the selection, handing their sets back;
- the priority slot (`TerrainStreaming::reprioritise`) gives every
  selected block `round(6000 − d)` and sets the ring to half the spread of
  the wanted, visible blocks' old priorities plus 6000. The original
  reaches it only through the terrain's method table (no direct caller; the
  frame hook does not call it), so it is not run every frame: the ring
  advances by one a frame that bakes and keeps its value otherwise;
- the selection's focus is the active camera with the lowest order moved
  10 m along its back axis and snapped to a 6 m block centre; blocks within
  44.4 m are selected and those within 15 m high, the list is sorted by the
  original's keys, and one block a frame is baked;
- a hidden selected block's key is its nearest room light's distance: the
  oriented box against the light's outward planes, the plane distance
  `(z + (y + x)) + w` less the box's support along the plane normal in the
  box's frame (its dot products with the box's axes), the largest non-negative clearance (0 when they overlap).
  The light's plane list itself is not in the pack; Exarust models a point
  light's reach as a box of its stored radius (1.25 radii, the falloff
  reach) — a labelled guess. The lights tested are the block's room's light
  list (`rooms::RoomLights`): each light joins the room of the cell it
  stands in and the rooms that own a light pass their lists through their
  portals, once, in room order; a room listing no light leaves the
  original's 65 000 cm. Exarust builds the lists from the lights' current
  places each time it ranks (a carried light changes rooms); a level
  without rooms ranks against every light.
`terrain::wireframe` draws a block's edges as lines when `EXA_TERRAIN_WIRE`
is set (the original's debug wireframe hook).

**Height query** (`TerrainGround`): the original's ground slot floors the
tile coordinate `width/2 + x/48 m`, takes tile slot `tz · width + tx`
anywhere in the grid's slots, floors the fraction times eight for the
6 m block, refuses blocks the level does not build, takes the bilinear
height of the four vertices around (its sums in the original's order; a
hole is tested as a byte, so the right-hand neighbour is not tested where it
starts the next byte), and reports no ground at or below −100 m.

Each layer's height, gloss and blend mask are packed into one texture, and
every layer texture is read through one repeating sampler. Each vertex's
over opacities sit in a per-tile texture of four RGBA layers, read
bilinearly between the vertices. The terrain's displacement self-shadowing
is effectively off in the original (its step is ~0).

**Blending** comes from the material's blend word (`0x44`):
- the source factor is bits 8–11 and the destination bits 0–3;
- `0x100` is opaque;
- every lit record whose source is ONE pre-scales its diffuse colour by
  the opacity, except in refraction mode (`0x300` = `0x200` in the
  plug-in-masked word, so only a type-1 record); the stored opacity is used,
  not a clamped one. The source nibble is the one after the late rewrite
  below. The blend's destination does
  not enter this test;
- a record whose destination nibble is 3 (the transparent class) is
  rewritten late to source ONE / destination ZERO, i.e. opaque, when it is
  in refraction mode (the loaded flags, whatever the type), or when its
  opacity is above 0.99, its fall-off amount below 0.01 and the loaded
  opacity flag (`0x400000`: the record's own, or an opacity layer's) is
  clear. The class it starts from is the one after the alpha-test rewrite
  (an alpha-tested word has already lost its destination nibble). The two
  compares widen the stored f32 to f64, so `0.99` stored as f32 passes the
  first and `0.01` passes the second; NaN fails both. The destination and
  source nibbles (bits 0–3 and 8–13) are cleared and source ONE is set,
  keeping bits 4–7 and 14 upward;
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
  a `missing.dds`. Exarust uses the men's set, or the women's when the
  race has it and the character's record is female (`look::skin_set`), as
  the inputs of the generated maps (below), not as the drawn maps.
- **Diffuse map**: the race's skin texture (`humandifm`, `humandiff` for
  women) times the character's skin tone. The original loads the race
  texture as data, so the product is taken on its stored, sRGB-encoded
  values; it renders the map with the sRGB encoding off into an sRGB
  texture, which drawing the skin decodes once. The tone multiplier comes
  from a 3 × 3 grid read bilinearly at the appearance's two tone values:
  0.29–0.64, (0.46, 0.47, 0.47) at the centre. The texture's own skin
  colour, sRGB (0.80, 0.65, 0.57), shows as linear (0.112, 0.076, 0.059) at
  the middle tone. Exarust reads the record's two tone words
  (`CharacterRecord::skin`) and computes the multiplier as the original does
  (`human_skin::tone_colour`): the nine tones are an HSV table turned into
  linear colours at start-up, interpolated per half of the grid, divided by
  the base texture's colour, scaled by 0.6, encoded and scaled by 1.4, with
  the engine's fast logarithm and exponential; it matches an instruction
  emulation bit for bit. A body without a record (the player, the duellist)
  keeps the middle tone.
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

The skin job also decides the record's darkening (the feral level when the
race's look bit `0x100` is set and the character is feral or aged), its
beard (`bearda01d/n.rfi` to `bearda06n.rfi`, from bits 20–23 of the
profession word, coloured by the hair picker's diffuse halfway to grey 0.3)
and which normal maps blend over the base map and by how much
(`look::normal_layers`: weight `i` drives physique map `i + 1` when over
0.01, the feral level added to the first, and the extra `nrmm5`, the men's
for both sexes, able to show for the feral level alone; a woman of a race
with only the men's set gets no layers). `look::skin_set` reads the race
look word, `look::beard_index` and `look::skin_darkening` the record.

The composite's shaders are reproduced on the CPU in
`exa_assets::skin_composite`: `chargen_d` multiplies the stored colour texel
by the tone, `chargen_s` packs `(1, min(0.8·gloss, 1), 0)`, `chargen_n`
adds `(an − bn)·weight` for each physique layer, and the beard's three
overlay programs blend the beard's alpha (its colour pass squares it and
uses the mip-3 alpha; its specular is `a³·0.4`; its normal replaces the
normal where the beard's colour alpha covers). The beard is drawn only over
the map's first quarter-by-quarter corner (the overlay quad spans 0…0.25 of
the target with the whole beard map on it). `chargen_n` binds at most five
layers and stops at the first amount not above zero. The callback fills the
colour target, then the normal target, then the surface target, each with
its beard overlay and then the apparel layers. `compose` runs them per
texel and stores the original's sRGB colour target and linear surface and
normal targets, its rows split over up to eight threads (same texels, same
result). The `chargen_*` programs bind no `UVOfs`: the record's two seed
offsets shift only the decomposition pass's `skindecomp` fetch.
`compose_with_decomp` runs the callback's decomposition branch
(`exa_assets::decomp`, the `chardecomp_d/_s/_n` passes): when the `Decomp`
amount is above zero the material first draws its mask pass into a
one-channel 8-bit target of the job's size (`humanseams` times `humandcmpw`
fetched at `UVOfs`, times the amount, smoothed twice), then replaces the
three base programs: the race's colour or gloss in `Tex0`, the mask in
`Tex1`, the job's tone as `SknCol`, and for the normal pass the base normal
and layers in `Tex0`..`Tex4` with the mask rebound at `Tex5` (so a fifth
layer reads the mask). The beard overlays draw on top and the apparel
layers last. The amount is the job's feral level (a race whose look
darkens, `look::skin_darkening`); the loader draws the pass whenever it is
above zero (Kade of `exanima01` is fully feral).

The running skin is the composite, not the race maps plus tint.
`SkinJob::composite_job` describes the job
(`exa_assets::SkinCompositeJob`), and `ModelLoader::composited_skin`
decodes the race's colour, gloss, base normal and layer maps and the
beard's colour and normal maps on the CPU, composes at the pixel-format
side (Exarust runs at the original's 512 default; 1024 at setting 3) and
uploads the colour target as sRGB and the surface and normal targets as
linear images with a mip chain. The material keeps the built-in skin's
subsurface scatter and packed-specular flags, sets its diffuse to white
(the maps carry the tone) and takes the three maps. The composite is built
once per distinct look and cached by the job's key: body stem, tone, side,
beard and normal layers (the seed does not change the maps). The dressing
pass generates it on the async compute pool and the merged skin run draws
the race maps until it is uploaded; it costs 14 ms at 512² and 124 ms at
1024² with the eight-thread split, so `exanima01`'s dressing stage stays
as long as the per-node draw's. A body whose race map or base normal does
not decode keeps the race maps plus tone. Not yet: the feral darkening
(the job keeps it as data; the shader does not apply it), the player's tone
(it is in the save, which is not shipped), the decomposition dirt layer in
the running path. Blood: see [Blood](#blood).

### Hair

The body builder adds the record's hair style from `CharExt.rpk` (style 1
is the first `hair …` entry, style 0 is bald) under the head part (bone
10), re-framing its roots on the head, when the race has hair (look bit
`0x10`) and the worn set's coverage union on the head slot lacks bit 1.
Roots whose name begins with `?` show only while that union has bit 0 (a
hood or helmet), the others only while it does not. The look rebuild
compares the built hair (none, `?` roots, full) with the wanted one and
keeps, removes or rebuilds it (`look::hair_cut`).

`exa_character::hair` loads the style once and parents its nodes to the
head by those rules. The hair material takes the record's colour:
`exa_assets::hair` reproduces the original's colour picker (five decoded
hue anchors, desaturation by the saturated value, a specular grey moved
in, the engine's sRGB chain, with its double-precision steps), and the
feral level lightens the colour, lowers the gloss (0.9 − 0.4·f) and thins
the strands ((1 − 0.65·f)²); the record's bit `0x10000` leaves the
specular level unboosted. The original swaps its hair material in only on
parts whose material word at payload offset `0xC` is over 0.5; the shipped
hair material (`harhps1`) holds 1.0 there, so every part is tinted. Not
yet: hair as cloth, the `HarHPC1` texture set bit `0x10000` selects (its
maps do not ship; `exa_character::look::hair_textures` names both sets),
and the creation scene's smoothing.

### Character assembly

The original dresses a character into one auto-skinned mesh: the body and
every visible garment piece are appended into a single mesh grouped by
material. `exa_character::dress` is the merge list, and
`exa_assets::merge::merge` builds the mesh from it. The running game does
this: `dress::assemble` builds the body tree from the built body and
garment models, re-frames each garment on the body's slot frames, plans
the merge and gives each step's built material group as a
`model::DressedPart`; `model::merge_dressed` turns the built parts back
into the file's vertex streams and merges them, and `model::run_mesh` makes
one sub-mesh per material run. Each run is drawn as its own entity bound to
the character's skeleton, the body's root part as joint 0; the body parts
the merged mesh replaced, and the body slots a garment covers, stop
drawing. Held items, hair and cloth keep
their own draw. No shipped material carries the occlusion mark and the
built meshes keep neither vertex slot masks nor triangle flags, so the cull
and occlusion passes do nothing on shipped data.

- **Order.** The worn set is planned in its layer-level order (the apparel
  cache inserts each garment by its highest occupied layer, outermost
  last). Each garment's chosen nodes are appended in slot order (1–18;
  the slot-tree walk before it only builds the skin weights), gated by the
  slot's select mask against the garment's visible mask.
  The body's own visible slots follow in the same slot order, then the overlays (meshes numbered
  past slot 18, their node flags `(flags & !0x30000) | 0xC0000`).
- **Cull bits.** A garment slot's triangles take `0x400000` when its second
  cull mask misses the visible mask, else `0x200000` when its first does,
  else none; the second test overwrites the first. Body slots take none.
  The bits are the slot append's cull argument (`MergePart::cull`): the
  append drops the garment triangles whose per-triangle flags meet them
  (`flags & cull != 0`) and appends only the vertices a kept triangle
  uses.
- **Occlusion alpha.** A garment part whose material has the occlusion
  mark makes its merged vertices opaque and every vertex on the border of
  the marked region transparent: a vertex fades when one of its marked
  corners' outgoing edge (to the triangle's next corner, by shared
  position) is not closed by a marked corner's incoming edge at the same
  vertex. Unmarked parts keep alpha 255.
- **Impact material.** Each body slot takes the hit-sound class of the
  garment that leaves the most of the slot's uncovered coverage word
  (compared as the original's 64-bit word). The body dynamics carry it per
  part; nothing plays it yet.

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
`sknhrtghoula` and the like, not the eyes). Hair draws with its own
program (above, Shader builder); the shipped hair record has no dirt mode.

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

Not yet: blood sprays, the attacker's own blood when it strikes with a
body part of its own (creatures), the translucent
program's blood ((0.5, 0.03, 0)), and dirt modes 1–3. Exarust never
re-dresses a character, so the original's clean meshes after an equipment
change do not arise yet.

### Blood on weapons

A weapon that deals a bleeding blow keeps the blood it draws, and a
second pass draws it over the weapon as a stain that runs down from the
tip, further as the weapon draws more.

**Blood** (`exa_combat::wounds::BladeBlood`, on the entity holding the
weapon's model): each bleeding blow it deals adds 0.5·R at the blow (R the
wound's size, [Blood](#blood)), kept within 0 to 1, whether or not the
wound itself waits or is dropped from a full queue. Nothing washes it off.
Each held weapon keeps its own; fists have none, so punches never bloody
them. The original frees a weapon's blood when its body leaves the
simulation and clears all of it at the end of a session or a respawn;
Exarust despawns the weapon with its holder and has no session end yet.

The original's own list (`exa_combat::weapon_blood::WeaponBlood`) keeps a
spot per source: the source, the amount it has left, and the stain's two
length terms (its two lengths over 80, and the square root of 80 over
them). A hidden source adds nothing; a source takes the first free spot or
grows the list; clearing the count empties it; clearing a spot frees it;
adding clamps the amount to 0 through 1. The shipped stain reads the
scalar above.

**Which meshes** (`exa_character::blood::WeaponStain`): the original
stains its weapon node's first sub-object. For a model whose root node
has meshes that is the root's first mesh (weapon items name 110 such
models, 103 of them in `Objects.rpk`; each has a root mesh standing above
its grip, 9 cm to 2.03 m, and the top of its stored bounding box is the
top of that mesh; 36 have more than one mesh, all tools and props such as
brooms and chisels, stained on their first); an assembled (factory)
weapon is merged by the original into one mesh, so every part is stained,
in the weapon's frame.

**Coordinates**, from the mesh's position in the weapon's frame in cm
(the frame the blade runs up, +Y from the grip), with L the tip's height
(the top of the stained node's meshes, or of the whole assembled weapon)
and a the blood:
- u = 0.025·x − 0.5: the blade's centre line at the gradient's middle
  column, repeating every 40 cm across;
- v = 0.0125·y + o, o = −(L/80)·(1 − √a·√(80/L)), so v = (y − L)/80 +
  √(a·L/80): v = 0 lies √(80·L·a) cm below the tip (34.7 cm of the
  60.05 cm short sword at a = 0.25, 69.3 cm at a = 1), and below that the
  gradient clamps to its white first row (no stain); past v = 1 its last.

**Gradient** `BldGr02D` (`Textures.rpk`, 256 × 512, BC1), loaded as data
(not sRGB, the original's loader flag), repeating across (its file flag)
and clamped along its height (no flag; [formats/RFI.md](formats/RFI.md)),
with the colour textures' LOD bias: white on its first rows, red on 14 %
of row 64, 33 % of row 128, 79 % of row 256 and 87 % of row 448, mostly
solid dark red (about sRGB (170, 55, 50)) toward the end. So the stain is
densest at the tip and breaks into separate specks toward its edge.

**Pass** (`exa_render::stain`, `stain.wesl`): drawn only while the weapon
holds more than 0.01; the texel faded to white by the object's fade (no
Exarust object fades: 0), written opaque and multiplied into the frame
(white leaves the weapon unchanged); depth writes off, the depth nudged
toward the camera by 0.000 005 of the depth range (the original's 0.000 01
on its −1…1 depth) so the overlay passes the test against its own weapon;
back faces culled; no shadows. Exarust draws it as a Bevy material with
the multiply blend, one material per stained mesh (no mesh is copied).

**Difference**: the original multiplies its 8-bit frame, in which a lit
value above 1 has already clamped; Exarust's float frame keeps it until
the post-processing clamps, so a specular highlight on a stained blade
can stay brighter (redder) than the original's.

**Evidence**: reverse engineering of `Exanima.exe` 0.9.5 (the add routine
and its 0.5, the clamp to 0…1, the registration's k1 = L/80 and k2 =
√(80/L) from the node's bounding box, the draw routine's threshold 0.01,
its uniforms (0.025, −0.5, 0.0125, o) and the node's first sub-object, the
depth nudge, the gradient's loader flags and the wrap and linear-upload
rules, the two GLSL sources), and the corpus (`BldGr02D`'s rows and flags,
the weapon models, pinned by tests). The span is the float 80.0 and its
inverse the double 0.0125 (`exa_combat::weapon_blood`).

### Blood sprays

Each bleeding blow sprays a short jet of blood droplets from the wound
(`exa_character::spray`; all lengths in cm, all rules at the 83 Hz body
tick). Droplets fly and fall but touch nothing: blood never reaches the
floor (the original has no floor blood from combat; its floor blood is
placed by the level designers).

**Scene node**: the original builds once a geometry node named
`_BloodSpray` with one sub-object using the built-in `_BloodPrt02` material
(texture `BldPrt02`, a 240-byte built-in record registered under that name),
the node flag word 0x1000, added to the scene's effect list, and the one
spray emitter. Exarust draws its droplets through one mesh and the resource
(`exa_character::spray`) and keeps the node's names and flag words as data
(`BloodSprays::scene_node`); nothing builds that node.

**Jets**: one per bleeding blow (R over 0.05 on a race that bleeds,
[Blood](#blood)), started at the blow even when the wound is dropped from
a full queue (`exa_combat::wounds::Spray`):
- at the contact point, fixed to the struck part's frame (where the
  body's last tick left it), so it rides with the limb;
- pointing against the striking point's swing (the blade tip's velocity),
  its frame's second axis along the jet, the first across it and the
  world's +Z, the third across both (a jet along ±Z keeps only its axis);
  the first two are made unit as the original's normalise does, by the
  processor's approximate reciprocal square root of (x² + z²) + y² (unit
  to about 12 bits), the third is their cross product as it comes. The
  frame is tied to the part by the transpose of the part's turn: each axis
  becomes its dots with the part's axes, (x·aₓ + y·a_y) + z·a_z, and the
  origin the same of its offset from the part's origin;
- strength 120·R, or 200 once R exceeds 1.67 (in double precision); it
  sends round(0.3 × strength) droplets, halves to even: 36·R droplets, 60
  at most (16 for the chest cut of R 0.457), and then stops; a new jet
  takes the first finished jet's place;
- dropped, all of them, when the world is torn down (the original's world
  reset sets the spray's source count to 0; Exarust runs it on a level
  change's teardown, a guess for the session routines that call it). The
  droplets already flying, the clock and the slot sizes stay.

**Emission**: the scene's sprays share one clock. Each tick it drops by 1
with probability ⅔; while it is below 1, every running jet sends a
droplet and the clock gains (83/40)·⅔ = 1.383 ticks. So a jet sends 40
droplets a second and lasts 0.9·R s (1.5 s at most).

**A droplet** starts at the jet's origin:
- velocity (cm/tick): the jet's frame applied to (u₁, u₂, u₃), u₁ and u₃
  uniform in ±0.25 (thousandths), u₂ in 0…1, times 48/83; plus 0.15 of how
  the jet's origin moved since its last droplet; averaged half and half
  with the jet's previous droplet's velocity. A new jet's "previous"
  velocity is twice its axis as the struck part's own frame sees it (the
  original seeds it after turning the axis into the part's frame), so the
  first droplets lean that way and the lean halves with each droplet;
- placed 0.8 of its velocity ahead of the origin and its previous position
  1.2 behind, so it leaves at twice that velocity: the first droplets of
  a jet fly at 2–2.6 cm/tick (1.7–2.1 m/s), later ones at about 1.2
  cm/tick at most plus 0.3 of the limb's own motion;
- it lives 62 ticks (83 × 0.75, truncated). At most 512 live at once (the
  original's pool; Exarust drops a droplet past it, where the original's
  ring would overrun).

**Each tick** every droplet ages a tick, the dead are dropped, and the rest
take a Verlet step: d = p − q, d.y += 0.25 × (−980.665/83²), p += d ×
0.95^(1/83). A droplet at rest falls 66.4 cm in the 61 ticks it moves.

**Drawing** (per frame, between the last two ticks, oldest first, one
world-space mesh): each droplet is a diamond facing the camera (the view
is the camera's forward axis):
- its long axis along its motion less 0.25 cm/tick up, so a slow droplet
  hangs down;
- half-width s + ℓ·t²·24, with s its ring slot's size (each of the 513
  slots draws 24·(0.5 + 0.5u) once, 12 to 24 cm), ℓ the sine between the
  motion and the view and t its age (0 to 1);
- stretched along the motion by (2 − 2/(σ + 1))·angle, σ the motion in
  cm/tick and the angle between it and the view;
- alpha over its life 0.5 → 1 → 0.5 → 0.25 → 0 at ages 0, ¼, ½, ¾, 1
  (bytes 128, 255, 128, 64, 0, interpolated and rounded, halves to
  even);
- UVs: one of the four splats of `BldPrt02` (`Textures.rpk`, 256², BC1,
  a 2 × 2 atlas of red spatter on white), under one of the eight
  symmetries of the square, as fires draw their wisps.

**Material** (`exa_render::spray`, the original's `_BloodPrt02`): the
texel (sRGB, clamped on both axes: neither the material's load flags
`0xF0` nor the file's `0x20A80000` carry a wrap bit; the colour textures'
LOD bias) mixed toward white by the droplet's alpha, written
opaque and multiplied into the frame (the original's ZERO, SRC_COLOR):
white leaves the scene unchanged and the splats darken it, so droplets
show over bodies, walls and floors and not against black. Unlit, both
faces, no depth writes, no shadows.

**Differences**: the original multiplies its 8-bit frame (lit values
already clamped at 1) and fades the droplets with its distance fog;
Exarust's float frame clamps in its post-processing, and its spray
material does not sample the level's fog. The original clears the sprays
at the end of a session and on respawn; Exarust has neither.

**Evidence**: reverse engineering of `Exanima.exe` 0.9.5 (the blood
branch's two jet calls and their strengths, the jet routine's life,
frame, part link and seed, the spray template and its derived values,
the particle tick, spawn, quad and colour routines with the template's
flags, the droplet material), and `BldPrt02`'s header (256², BC1, the
file's wrap flags); the rules are pinned by tests.

## Smoke and mist

`exa_render::smoke` is the original's `shadesmk_f` draw. The original
compiles `shading_v` + `shadesmk_f` once per state key (the material
state's 64-bit word under `0x0003_C00F_0048_0008`), sets `UVOfst =
(0.5, t + 0.5, t)` with `t` = the parent node's scale plus the scene time's
milliseconds mod 80 000 over 80 000 (summed in double precision), and draws
the mesh; with the supersampled quad passes on it repeats the draw in each
pass once per sub-view the pass draws (the level's first-pass count, at most
four), each with that sub-view's viewport and matrix (`smoke::pass_draws`).
`smoke::ProgramCache` holds one program per key; a failed build is kept as
−1 and the mesh is skipped. The program (`shadesmk.wesl`, line
for line): `a = |N·V|·1.5 − 1`;
with an opacity texture `a += A(uv + (0, z))·A(uv + (0, z) + (x, y))` for
`UVOfst = (x, y, z)`, else `a += 0.25`; then `a = (a − 1 + vertex
alpha)·0.8` over a black colour. The original's fog and dither snippets
follow; Exarust's surfaces have neither. Nothing spawns a smoke mesh yet.

## Pixel-space 2-D batcher

`exa_render::quads` is the data path of the original's immediate-draw
helpers. `begin` takes the viewport's pixel rectangle and sets `1/width`
and `1/height`; a vertex maps to `((x + tx)·sx, (ty − y)·sy)`, where `tx` is
the current translation's x and `ty` the viewport height less its y, both
taken at `begin`. A translation adds to the current one and saves it on a
16-level stack (the top slot is overwritten past it), and a pop restores
the level below. A texture coordinate (or four, by default `(0,0), (1,0),
(1,1), (0,1)`) belongs to the next vertex slot; a colour is RGBA bytes.
`flush` draws every whole four vertices as two triangles from the index
template `(0, 1, 2, 0, 2, 3)`; the line draw takes vertex pairs. The
vertices are 0..1, drawn through the projection `2x − 1, 2y − 1` with depth
scaled by −0.001; a raw pixel vertex uses that projection scaled by `sx`
and `−sy` and moved by the translation. The six built-in vertex programs
are kept verbatim with the format-word choice and the attribute locations
(`vPos` 0, `vTex` 1, `vNrm` 2, `vCol` 3, `vTx2` 4, `vTng` 6, `vDlt` 7).
Nothing draws a batch yet.

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
- Sectors fall on the level grid's 6 m cells: cell i is centred at
  ((i + 0.5) − ⌊size/2⌋) × 6 m, the size halved and rounded down as the
  original's integer division does (an odd grid's cells sit 3 m off a
  multiple of 6 m). The manager lays its blocks out **once per level**, as
  the original does: the bounding box of the level grid's built cells
  (plane A's `0x80000`, the border skipped; a later row reaches the bottom
  edge when it moves the left edge, has a built cell inside the current
  span, both ends included, or one right of it), rounded up to whole blocks
  of eight cells a side and slid back inside the grid; every eight-by-eight
  block with a built cell becomes a block. A level without a sector grid
  gets the original's synthetic 8 × 8 grid and its one block at cells
  (3, 3). The block's top is the level record's highest point (`0x98`),
  truncated to whole voxels.
- The original keeps up to nine blocks resident around a point 20 m in
  front of the camera and steps them all; Exarust does too, and binds as
  the camera's block (the one it warms first and the loading screen waits
  for) the block the camera's own cell falls in (the original's per-object lookup floors the offset
  from the block grid's edge, counted from the level grid's half size in
  float, over 48 m), or, when that one has no geometry, the block nearest
  the focus 20 m ahead measured from its 24 m half extent, as the
  residency sort ranks them. The residency itself runs as the original's
  does (`exa_world::gi_residency`): the first nine blocks in creation order
  hold the light textures; every frame each **built** block's distance from
  the focus is `max(|f − c| − 2400, 0)` per axis in centimetres (a NaN axis
  is not clamped), squared and summed Z first, and a stable insertion sort
  moves a block forward only past a strictly farther one, so unbuilt blocks
  (still at 0) lead in list order; the builder takes the first unbuilt or
  dirty block in that order, and a built block among the first nine
  without a texture takes the farthest textured block's beyond them. While
  the camera's block stands, Exarust builds the block the residency names
  next in the background; finished occupancies are kept for the whole
  level, as the original keeps them, so walking into a neighbouring block
  swaps its volume in from the cache. One build runs at a time; a camera
  block that needs building cancels a background build (the residency asks
  for that block again), and a newer camera pick waits for a camera build.
  Exarust builds the camera's block before the residency's order, so a
  load is lit first where the player stands. The block already on screen stays bound
  while a newly built one warms behind it, so a move never reveals a cold
  block's black patches or propagation halo; the warm block replaces it in
  one step. A level change drops both blocks. When a load ends, the block
  is rebuilt around the camera's starting view if it is not already there.
  The renderer keeps up to nine blocks' light and scratch textures
  (`GiTextureCache`), as the original owns nine: a block the camera comes
  back to reuses the light it accumulated and skips the warm-up, and its
  occupancy is uploaded whole again from the world's cells. Every block
  the residency steps besides the camera's (built, holding one of the nine
  textures) is published as a resident neighbour (`GiNeighbourVolumes`, in
  layout order, left alone while it names the same builds) and gets its
  textures (`GiNeighbours`), its own sector marks and cell lights, its
  margin copies and one light step a frame after the camera block's. A
  neighbour without kept light owes the volume's warm-up; the neighbours
  share 16 extra steps a frame [adaptation: the original's texture objects
  are handed between blocks with their old light and steps start from
  there]. A level change empties the cache.
- **Per-object pick.** The original looks the block up for every object
  it draws (each terrain and grass block once): the object's position
  less the block grid's low edge, in centimetres, times 1/4800, floored on
  each ground axis, names the block, whose light texture and `VXGIPos` it
  binds; a position off the grid, or a block without a light texture,
  binds texture 0, which reads (0, 0, 0, 1). The surface program then
  samples with no box test (a fragment beyond the block reads its clamped
  edge; the texture keeps the engine's clamp wrap). Exarust binds nine
  slots in every lit material (`GiSlots`: the camera's block in slot 0,
  the neighbours after it, each slot's sample origin and block-grid cell
  `GiPick`, and the grid's edge) and picks per draw in the fragment
  shader from the mesh's world origin (`gi_slot`); terrain takes the
  centre of the 6 m terrain block under the fragment (the tile is eight
  blocks a side), and plant chunks stand at their block's centre. An
  object no slot holds samples (0, 0, 0, 1). The focus-centred block of a
  grid with no built cell is off the grid and lights every object. Which
  point the original's lit-material builder passes for a node was not
  traced to its field [guess: the node's origin].
- Surfaces **sample** the volume with the original's per-object transform:
  `texture(p·scale + offset)` with `p` in world centimetres, the offset's X
  and Z half a texture minus the block's centre over its width and its Y
  the block's bottom slice over the depth, and the scale
  (1/(W·fc), 1/(D·fc), 1/(H·fc)). A surface reads half a voxel up (the
  offset's Y carries +0.5), as the original's per-object lookup does,
  unless the node's flags2 keeps `0x800000`; the terrain and grass callers
  pass the flag that drops it and read the voxel's bottom instead (Exarust
  does not model the per-node `0x800000` exception). Both are `GiBlockLayout::sample_transform` /
  `sample_at`, and the material paths take the same origins from
  `GiTextures::sample_origin` and `sample_origin_floor`.
- The top slice holds 26 and the slice below 5, a soft lid.
- Terrain comes first: in each interior column (not the margins), voxels
  below the terrain surface hold 20, and the surface voxel its covered
  fraction of 20. The surface height in voxels is truncated towards zero:
  a column more than a whole voxel below slice 0 is skipped (one less than a
  voxel below counts as slice 0, its negative fraction stored as a wrapped
  byte). A surface at or above the top slice clamps to it: no surface byte
  is written and slices 0..D−2 hold 20, so the lid's 5 becomes 20 and only
  the top slice's 26 stays. The column's lowest slice is the clamped height.
- Every mesh's stored **GI sample points** (chunk `0x3D0CEC05`: a spacing
  word, a count, then object-space points in cm) are transformed to the
  world and splatted:
  - trilinearly with weight 20, each cell's share rounded on its own, the
    sum saturating at 255; a mesh marked thin (`GiThinMesh`: the
    original's mesh set-up flags a surface thin when its material's render
    class has `0x4000`, or it has an opacity layer and an opacity above
    0.75, and the splat reads the first surface's) weighs a quarter,
    and a centred one (`GiCentredMesh`, the node's flags2 `0x10`, which the
    model reader keeps) adds half a voxel in height and leaves its column's
    lowest slice alone;
  - points within a voxel of the block's sides, or in its top slice, are
    dropped;
  - the spacing word is unused. Wall pieces store 100, and their points
    land one per voxel centre, making solid slabs.
- Many wall sets (the menu's `GEWS_UWA02`) store no points, and the original
  generates none for walls. Those walls block nothing.
- **Floor layer**: each tile-grid cell's floor meshes (the floor set's
  pieces and the wall pieces' `+F` strips) are merged, as the original
  merges them into the cell's `+Floor` mesh, and the merged floor goes
  through the original's floor heightfield
  (`exa_physics::voxel::floor_points`) once, when the level's GI inputs are
  collected, in the cell's own space (origin at the cell centre) and in
  centimetres:
  - the x low edge is the first of −300/−150/0/150 cm with
    `low < edge + 75` (default −300), the x high edge the first of
    300/150/0/−150 cm, downwards, with `edge − 75 < high` (default −150); y
    starts at −20…0 cm and z at −300…−300 cm; every edge then widens to the
    floor's bounds;
  - low edges snap to `round(v/20 − 0.1)·20`, high edges to
    `round(v/20 + 0.1)·20` (ties to even), and the y low drops 20 cm more;
  - one ray straight down every 10 cm column, from 10 cm above the high
    edge; the highest upward-facing triangle under it wins, edges inside,
    and the ray must land within the bounds;
  - the 10 cm cell under the surface is marked (`n_y − round(t/10)`);
  - every 2 × 2 × 2 block of fine cells summing past 1019 (its four columns
    marked inside it) gives a point at its centre on a fixed 20 cm step.
    With an odd number of fine rows the original's walk drifts one row back
    per block slice; that is kept (a 20 cm parameter always gives an even
    count).

  The meshes' render triangles stand in for the original's collision
  triangles, and the normal is normalised exactly where the original uses
  the hardware's approximate reciprocal square root (the plane is rescaled
  by its y, so only rounding differs).
  A flat floor at height 0 fills the slice from −20 to 0 cm.
- A floor-set node gets runtime points at load where the original's
  floor-set loader generates them: reached from the centre-floor variants
  (`+FM…`), whose direct children named `#…`, `+…` or `[…]` are skipped
  with their subtrees (deeper nodes are not name-tested), with bit
  `0x10000` or bit `0x8` of its second flag word set. The general surface
  generator runs on its own triangles in centimetres at parameter 20 (a
  4 cm step) and greedy-covers them
  (`exa_physics::voxel::generated_surface_points`), replacing any stored
  chunk; the sphere query is a triangle tree over the same triangles,
  standing in for the node's collision query. A flagged instance's master
  is generated and its points shared. Wall and special-tile pieces take
  none. The original also generates points for a character's body parts
  when their physics bodies are made; Exarust's characters carry no GI
  points, so that trigger is not run.
- Each column remembers the lowest slice a point fell in. A sector's floor
  is the lowest over all its 30 × 30 columns.

**Propagation**, one step per frame. A new block first gets a warm-up,
600 steps by default, run 16 steps a frame (all of them in one frame stalled
the window for seconds); the loading screen waits for the first block, and a
rebuilt block warms behind the one on screen and is swapped in only when its
warm-up is complete. The light is 8-bit sRGB RGB, with A = 1 − 5·occ.

Only slices from a group's lowest sector floor up to the second-highest are
updated. The manager marks the level cells to draw each frame in two
passes over the same window: cells 1 to size − 2 within six of the focus
cell (the focus point is the camera's position 20 m along its view), at
most 36 cells squared from it and built in plane A's open flag `0x80000`.
The first pass marks such a cell when its box meets the camera's convex
volume. The box is 300 cm either side of the cell's centre in X and Z and
spans from the cell's floor slice to the block's top in Y (a cell without
geometry, floor 255, gets a negative half height). The test runs plane by
plane, in centimetres: the box is out when its centre lies more than
848.53 cm (600·√2) beyond a plane, or when it lies beyond the plane by more
than the box reaches towards it, Σ n·h with each half extent h taking the
normal's sign bit on top of its own (so a negative height reaches away
from the plane). The second pass keeps a cell of the window that passes
the same reach and plane A tests when its own first mark or a neighbour's
is set; the neighbours are read at the cell's row-major index ± 1 and
± the grid's height (the original's stride, the row length only on a
square grid). Sectors left unmarked, sectors with no points at all, the
margins and the top slice are cleared to zero every step (the top slice is
drawn instead when the environment record has flag `0x20`, as on the
outdoor terrain arenas: the original then draws `D − floor` slices rather
than `D − 1 − floor`, `drawn_slices`), as the original
clears the scratch target and draws only the groups: they stay dark and
absorb light that reaches them through open walls. Voxel lights still add
into them, as the original's additive pass follows the clear. A level
without a sector grid draws all 64 sectors as one group from slice 0 (the
original does not set that group's floor; it is 0 until a grid level has
run [inf]), whatever the marks.

For each updated voxel, with occ normalised. The light texture is read at
the voxel's **own slice centre**: the original's quads put their vertex r
at −0.5/depth and the vertex shader subtracts (instance + LyrOfs)/depth, so
layer L reads r = (L + 0.5)/depth and only the ±1.33-voxel taps blend the
neighbouring slices. The shading maths uses the volume's height divided by
its width, at the slice's middle ((slice + 0.5)/width), as the original's
`ZTrn` maps it:
- open = max(1 − 13·occ, 0);
- for the eight diagonal neighbours n at ±1.33 voxels:
  - a = open(n);
  - P = light(n) · a² (· a with the sky light);
  - sum += mix(old, P, a);
- open fades out over the lowest slices: · (1 − 2^(−16·r)), r the slice
  centre over the block's slice count;
- k = min(8·Σa, 1) · (0.3·open + 0.699);
- light = sum·k/8 + base·open + Σ lights;
- base = level record `0x30` (as stored) × 0.001, per channel, or × 0.002
  below shading quality 4 (`GiSettings::high_quality`, fed every frame from
  the settings screen's `ShadingQuality`; the same flag drops the manager's
  light share from 0.1 to 0.1 × 0.9 (taken in double precision and narrowed:
  0.09's nearest float), which only sources
  outside the GI manager read — `gi::injection_share`);
- each point light adds
  lin(colour)·intensity·multiplier·(1 − dim)²·0.15·0.01 · (1 − d/R)⁴ ·
  open · cube shadow · shaping. The point light's uniforms are the
  original's `Light[3i..3i+2]`: the position in texture units with 1/(R·s)
  in w, then the colour with the cube lookup's depth scale in w, then the
  shaping (Exarust packs it in x, y, z with the sun flag in w; the original
  keeps it in y, z, w). The cube lookup is the cached shadow pool's linear depth
  compared against d·(bias + 0.94)/R + 0.01 along (the point − the light),
  the scale worked out in double precision (((bias + 0.94)/far) × 5120).
  The bias is the shadow detail plus the light record's own quality offset
  (0, +1, −1 or −2), capped at 6, times 0.005, or 0.025 above 5: the
  retail setting's detail 5 gives 0.025, so the compare is d·0.965/R (q = 4
  gives 0.02). A light without its range flag has the 3.4e38 far plane, so
  its compare value stays at 0.01 and, against Exarust's distance / R
  faces, its lookup is lit [guess: the original draws its faces against the
  same far plane, so they hold ≈ 0 and its unranged taps would be shadowed
  wherever geometry is; not confirmed in a running game]. The original binds light i's cube map to its
  own `Shadow<i>` sampler; Exarust's shared pool carries the slot instead;
- the manager's sky light joins each drawn sector's group first and adds
  colour·3·0.3·0.01 · visibility · open, where the colour is the scene
  environment's record fog colour as the frame leaves it (the day record's
  fog × 0.375, plus the lightning blue during a strike; zero without a sky)
  and 3 is the manager's constant, then three times the 0.1 injection
  share; it is left out when the scaled colour's third component is at
  most 1.19e−7. The light shines straight down, with its own 1024² map of
  depth reach 1600, whose first comparison alone counts under a sky (its
  set-up, `sky_shadow_setup`: two depth fields both 1600, a softness of
  (1024·10)·0.5 and a view that keeps X and sends the scene's Y to −Z and Z
  to −Y).
  Exarust stands the volume's columns in for that map [adaptation]: a voxel
  sees the sky at or above its column's floor, the slice above the
  column's highest voxel with an occupancy byte of 10 or more (openness
  under one half), uploaded once per block (moving meshes do not move it).
  The colour is used as the frame keeps it [guess: the light node's own
  activation was not followed]. The scene's sun light does not reach the
  volume (Exarust injected it here before, with the same factor 3: a
  misreading of this light). Under a sky the original's GI shading takes
  an outdoor curve, (1 − a)²·0.08/(a + 0.035) + a and the environment cube
  × `Ambient.w`·5, which Exarust does not have yet (it keeps the indoor
  curve), so the outdoor GI ambient is fainter than the original's;
- a light's shading is (2^((ly + (1 − ly)·y)·z) − 1)/(2^z − 1), ly the
  light's vertical direction; unshaped lights carry (1, 1, 1);
- a registered shape adds its fragment body's colour per voxel over the
  manager's clipped box (`shape_draw`) and slice range, after the point
  lights and before the voxel lights. The pass evaluates the six bodies
  (`gi_dome`, `gi_cuboid`, `gi_bubble`, `gi_telebubble`, `gi_gradline`,
  `gi_multipoint`) with the original's operation order and GL comparison
  semantics: the shells over the voxel's world centimetres through the
  shape's `bMat`, the line and multi-point bodies in the volume's units
  (the block's width is 1: positions given in world centimetres are scaled
  by one over the width, and `Atten` is the original's, so a blast's 5000
  reaches about 72 cm in a 51.2 m block); a multi-point shape carries up to
  16 points. The shape's own `Tex0` is not modelled — the pass binds one
  white texel for every shape [guess]. The manager's own shape chain (the
  list, the box, the quad and the shared uniforms) is in
  `gi_shape`/`shape_draw`. The force blast, the force torrent and the
  teleporter bubble register theirs while they live
  (`exa_game::thaum::effect_light`, light scale 0.09); the other emissives
  have no live effect yet;
- the original's program carries two matrix-shadow light slots (`SdwMat`,
  `SdwMat2` for the first two lights whose shadow is one 2D map, i.e. the
  sky light and spots); before drawing a group it moves the first such light at
  index 2 or later to slot 1 when the sky light owns slot 0, or the first at
  index 1 or later to slot 0 without it (its scan reaches the last
  light, as its do-while does; `plan_light_slots`). Exarust has no spot
  lights and the sky light is always slot 0, so a group keeps its order;
  every point light carries its cube slot instead of a fixed `Shadow<i>`
  binding; the light uniform's position is formed in the original's
  centimetres: `(p - block centre)*(1/(W*fc)) + 0.5`, the height
  `below/W + y*(1/(W*fc))`, and `w = 1/(R*(1/(W*fc)))`;
- the lights injected are the sector's own: each sector's 6 m cell lists the
  lights whose flood windows reached it (up to 12), with the sky light ahead of
  them when it is up (it counts towards the twelve). A spot-shaped light is
  dropped from a sector when it no longer carries down to the middle of the
  sector's floor slice: its horizontal offsets from the cell's centre count
  as zero within 300 cm (or when not a number) and are pulled 300 cm in
  beyond, c = dy / √(dy² + (dz² + dx²)), and it stays only while
  c + (1 − c)·y > 1.19209e−7 (bits `0x33ffffd7`). A marked sector with
  geometry joins the first group holding the same lights (as many, each
  found, in any order) whose floor lies within two slices, and lowers the
  group's floor; otherwise it starts one. Two passes then fold a later group
  into an earlier one when they share a light (or both have none), their
  union stays within 12 lights, and the merged group's fill estimate
  (lights + 2) × sectors × ((depth − 1) − floor) + 80 is below the two
  apart (32-bit sums). A merge appends the later group's new lights and
  takes the lower floor. Each group draws its sectors with its own light
  list; a sector sees no other light, so light does not cross a wall into
  another room's injection. Lights the injection does not know are dropped
  after the grouping, as the original drops a light without its GI flag. A
  light without the range flag floods with the original's unlimited reach
  (walls and the flood's stack limit stop it) and still injects over its
  stored radius: the injection reads the record's radius whatever the flag.
  A level without a wall grid draws the scene's lights (the first twelve by
  id [guess: the original's list order]) in its one group.

**Manager rules kept as tested functions or partial wiring**
(`exa_render::gi`, `exa_world::gi_volume`):
- the three block presets: 5 cm voxels give 32 × 32 × 64, 20 cm 256 × 256 ×
  64 (128 above 12.8 m), and any other value 128 × 128 × 16 at 40 cm.
  Exarust ships the 20 cm preset only (`preset`);
- the eight corner offsets (1.33 voxels, X fastest, then Z, then height,
  w = 1/8; each axis formed in double precision and rounded once to f32 as
  the manager's init does) and the shared quad index buffer
  (`corner_offsets`, `quad_indices`);
- the manager's three linked programs and their binding tables
  (`GiPass::layout`): `gi_inject_v` + `gi_inject_g` with `gi_inject_f`
  (`Light`, `VSmp`, `ZTrn`, `LyrOfs`, `BaseAmb`, `WrlMat`, `SdwMat`,
  `SdwMat2`, `SliceSz`, `First`, `MPrj`; light i's `Shadow<i>` on unit
  2 + i, `LVox` on 0 and `OVox` on 1), with `gi_emission` (the voxel lights;
  `SliceSz`, `ZTrn`, `First`, `Light`, and only `OVox` bound, on 1), and with
  `gi_blit_f` (`UVOfs`, `SliceSz`, `LVox` on 0); Exarust's WESL pass replaces
  them but keeps the same inputs;
- the emissive shape list: registration appends, unregistration swaps the
  last entry over each match, and the draw box rounds the centre to
  voxels (ties to even) and the half extents up, clips the columns to
  voxels 8 and 248 and draws from centre − half height for 2·half + 1
  slices, only for a centre above slice 1 (`GiShapes`, `shape_draw`). The
  manager's side of the draw is kept with it: the program each shape links
  (`GiShapeProgram`: `gi_dome`, `gi_cuboid`, `gi_bubble`, `gi_telebubble`,
  `gi_gradline`, `gi_multipoint`, built with `gi_inject_v` and
  `gi_inject_g`), the four corners of a slice's quad in texture units
  (`shape_quad`, r = −(first + 0.5)/depth with each instance adding its
  layer) and the program-wide uniforms `SliceSz` = 1/depth and `ZTrn` =
  (−D/W, D/W) (`shape_uniforms`). The six fragment bodies are the original's
  own `shading.flb` programs, ported line for line and evaluated per voxel
  (`exa_render::gi_shape`: a shape's fragment reads only its own uniforms and
  the voxel's position, so a per-voxel evaluation is what its quad-per-slice
  draw computes): the dome's ridge texture, 70-unit cap and 6× shell weight (the bodies' constants are in the original's centimetres, the space `bMat` maps a voxel into), the
  cuboid's `max(|p| − bExt, 0)` falloff at 0.0002, the bubble's cubed texture
  and vertical fade, the telebubble's three-plane texture and pulse (its
  squared shell term is computed and overwritten, as in the original), the
  gradline's two-ended gradient with its point light, and the multipoint's
  points at a quarter (its occlusion term is computed and unused). A shape
  entity's `GiShape` component lists and unlists it, as the effects'
  constructors and destructors do. No gameplay effect registers one yet;
- a moving GI mesh takes its old points out (weight −20) and its new ones
  in (+20) over a box padded 1.5 voxels each side, rounded ties to even in
  world voxels (`Occupancy::update_dynamic`; the original pads the
  object's oriented bounds, Exarust the points' extent [adaptation]). A
  GI mesh the level's source collected is followed every frame
  (`GiMoved` keeps its source and last frames). Nothing runs while a block
  build is in flight, as the original returns while its build count is above
  zero; the old points come out before the new go in, and only the built
  blocks the old and new padded boxes meet are edited (the original compares
  integer voxel boxes, Exarust world boxes [adaptation]). A block built after
  a mesh moved holds it at the source frame, so its first pass moves it to
  the last one (the original's builder reads the current frames). The
  changed box is uploaded as a sub-rectangle (`GiOccupancyPatch`: the region,
  its end kept one voxel short of the size as the original's upload keeps
  it, and its bytes from `sub_upload`) into the bound block and one warming
  behind it; the world drains the patches every frame. A whole-block upload
  stands in when a texture is created;
- a neighbour block's margin copy (into every stepping block), from the
  original's fixed table of texture fractions: on the neighbour's side the strip 0..1/32 (or
  31/32..1) with the UV offset ±15/16, on the other axis the interior
  1/32..31/32; every slice but the top, and the
  destination quad's four corners with the original's base z of
  −0.5/depth (`margin_blit`, `margin_quad`). The pass runs every step of
  every stepping block for each stepping block beside it: the step clears
  the margins and each copy fills one, reading the neighbour's sRGB light
  at the same world texel (its own coordinates plus the offset) and
  re-encoding it into the block's target. The copies' bind groups are kept
  while both blocks keep their textures. A block warming behind the
  camera's takes no copies; once it is bound it takes them. A neighbour
  that does not step (beyond nine, or not yet built) leaves its margin
  dark, as the original's untextured block does.

**Voxel lights** (`0x3D0C` nodes: glowing items and a few level nodes)
add straight into the volume, after the propagation, every step and in
the warm-up:
- colour C = lin(rgb)·√intensity·3 (the original's linear workflow; 0.6
  in double precision on its legacy path), × 0.1 per step. The node keeps
  its base colour, current intensity, base intensity, base radius and
  active radius (copied from the base by activation), plus a flag word
  that marks a colour update and an active light. A clone copies the
  master's colour, intensities, radii and derived linear colour. An item whose model section sets
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
- alpha is left alone, and the stored radius is unused. The voxel corner's
  nearest voxel rounds ties to even (the processor's conversion); Exarust
  adds the term to every voxel the box covers, cleared ones included, and
  keeps up to 64 voxel lights per block, nearest its centre first, apart
  from the point lights.

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

## Environment and weather

The original's environment plug-in (`exa_world::scene_env`, applied by
`exa_world::environment`) keeps the level's indoor values or an open sky,
and the day cycle, fog, ambient, clouds and weather that follow from them.
Much of its arithmetic is double precision between single-precision
fields, and Exarust follows the instructions there, not the decompiler's
reading.

**Constructor.** Sun azimuth −0.21 and time of day 0.7516 (0 is midnight,
0.5 noon); sRGB sun colour 0.75; a global light of sRGB 0.75 at strength
0.6; ambient 0.314; fill 0.04; fog colour 0.5 grey; clear colour
transparent black; fog distance 1, density 0.01 and far plane 50 000 cm,
so the fog spans 50 100 cm (501 m); cloudiness 1.5; the sun mode is auto,
the sky and the fog are off; the sky map is 32, 64 or 96 texels by render
quality.

**The sun's path.** Without a sky the sun's direction comes from the
level's altitude and azimuth: θ = (1 − altitude)·π, φ = −azimuth·π (both
scaled in double precision), and (sin θ sin φ, cos θ, sin θ cos φ), the
cosines in double precision. Under a sky the plug-in builds a rotation from
the 45° latitude, the time of day · 2π and −azimuth·π, puts the sun on a
path at 180° with the ecliptic's 23.44° tilt and the season at its quarter
point (no declination), and the moon on a path at 0° with 0.3 · 28.58°.
The daylight factor is arcsin(sun.y)/π + 0.5 with the engine's cheap arc
sine, `π/2 − (π/2 − (0.2075 − 0.0535·x)·x)·√(1 − x)` for x ≥ 0 and its
mirror below (asin 0.5 comes out 0.52398). At 0.44 and above (a double
comparison) the sun is chosen, below it the moon. The chosen body is
lifted off the horizon (y = y/2 + 0.2 when below 0.4, then y = (1 + y)/2
above 0.44), each time renormalised with the processor's approximate
reciprocal square root. The scene's daylight scale is (daylight − 0.47)·3.
Without a sky the daylight factor keeps its last value. The azimuth and
time of day come from the level record when it has them.

**The day record.** The plug-in holds a track of six environment records
entered at climbing positions in 0…1. A value at or below the first entry
is that record; between two entries the two are blended by the position
between their keys; at or above the last key the fifth and sixth blend at
1. Each record carries a sun colour, a fill colour, a fog colour, a sky
cube tint, a sky colour away from and towards the sun, and further colour
slots. Blending leaves the record's key alone, and the fill, fog and one
further colour are linearised after the blend when colours are managed in
linear light (the others are linearised once when the table is set up).
The track is the executable's constant table, six records keyed 0.38,
0.44, 0.47, 0.54, 0.62 and 0.70 that the environment unit sets up at
start-up. The pass has an optional step that linearises every colour of a
record (a, b, the sun, the fill, the two sky colours, i, j, k, l, m, n and o
when its first channel is positive) and scales the two sky colours, i, j and
k's colour by 0.75; that step hangs on a flag of its own that nothing in the
executable sets, so the shipped game keeps the table in sRGB and instead
linearises the frame's horizon colour (below). The pass always gives a
record whose o is black an o a quarter of the way from its away sky colour
to j (no shipped record needs it). Every scene starts with that track
(`EnvironmentTrack::day` in `exa_world::scene_env`), so the sun placement
takes the record's sun, fill and fog colours: the outdoor fog is the
record's (0.0236 red at the default time, 0.375 of the fog colour
linearised by the blend) rather than a stand-in haze, and the `_Sunlight`
colour is the record's sun colour as stored (0.33 red at the default
time).

**Clouds.** A cloudiness above the clear 1.5, by a factor
(cloudiness − 1.5)·0.4 clamped to 0.9 (a NaN counts as clear), desaturates
the record's colours towards their luminance (0.299, 0.587, 0.114) and
scales them by powers of f = dim²·0.8 + 0.2 and √dim, slot by slot (the
sun's colour by f⁴, the fog's not at all, the sky colours by f², and so
on). The same factor dims the sun light by cloud·2, and a sun within 0.1
of the daylight threshold brightens the light again by
1 − |daylight − 0.44|·10.

**Fog.** The span is distance² · far plane + 100, from a start of 0. The
per-frame fog starts at the stored start and ends at the span; a cloudy sky
moves the start in by (1.5 − cloudiness)² · span · 0.01 and shortens the
span by 1 − (1.5 − cloudiness)²·0.7, and a sun below a daylight of 0.54
narrows it further by (1 − √f·(1 − low))·f with low = 1 − (0.54 − daylight)·4
held at 0.15 or more (a comparison, so a NaN stays NaN). The scene's
flash-clear switch closes the fog to −0.8…0.1 of the span. While a strike
runs, the record's fog colour gets the lightning blue (0.2, 0.3, 1.0), as
stored, added by I/(I + 2)·3 for the strike's intensity I; this is the only
thing a strike changes (the clear, the sun, the ambient and the fog's reach
stay as they were, and the original has no exposure step to move). The
intensity starts at zero, steps by (0.2 + 0.1·r) and then by ±(0.1…0.3)
every 50…89 ms, scaled by the strike's attenuation (1 − reach·0.0002)² and
held at zero or above, and the strike ends 100…599 ms after it began. The frame
tints the record's colour whether or not the level fogs. The renderer's
uniform build reads neither the weather's near nor its far value (where
they go is not traced).

**World fog.** The world programs' fog uniform `FogColr` is the frame's
scratch colour (below) with one over the *stored* span in its fourth slot.
Their `fog` snippet (fog mode 1, below the voxel-march quality) mixes the
surface, after its emissive term, by b = min(d · FogColr.w, 1) for the eye
distance d towards FogColr.rgb · (amb · 0.5 + 0.5), where `amb` is the
voxel GI's light before its environment-cube factor outdoors and the
ambient indoors; an unlit surface with only an opacity map has its alpha
scaled by 1 − b. Exarust hands the scratch colour and the span (start 0,
in metres) to the cameras' linear `DistanceFog`, and the world material
(`exa.wesl`, every lit program including hair) reads Bevy's fog uniform
and applies the original's mix with its own ambient for `amb` (its GI
ambient has no cube factor). So a clear noon fogs over about 250 m towards
the horizon colour, a storm over about 100 m; indoor levels have no fog.
The 16-step voxel fog march of the top shading quality, and fog on the
terrain's own draw and the effect shaders, are not ported.

**The record fog colour lights the GI.** Its one reader is the voxel GI
manager: times the manager's constant 3 it is the colour of a directional
light that shines straight down, with its own 1024² depth map, and that
light is the first light of every drawn GI sector (see
[Voxel GI](#voxel-gi)). That is where a lightning strike shows in the
original: the outdoor GI, and with it the ambient of everything under the
sky, flashes blue. With `--no-gi` a strike shows nowhere, in the original's
terms as in Exarust's.

**Clear colour and sky.** The clear colour is the level's background. With
fog on it can be the fog colour instead (the original tests a high bit of
the sky map's word, which its constructor values never carry). Under a sky
the colour between the record's two sky colours, weighted
0.5 − 0.5·(light direction · view) (0.5 without a sun light), times the sky
fog colour and 2, is the horizon colour; without a sky the fog colour is.
The flash-clear switch clears with the scratch colour times the scene's
flash tint (zero from the constructor), halved. The original reads that
switch in one place and nothing in the executable sets it, so it is off in
the shipped game; a lightning strike does not turn it on (Exarust's earlier
stand-in did, and cleared each strike to black). `EXA_LIGHTNING_FRAME=<n>`
forces a strike 316 cm from the view from frame `n` on whenever none is
running, so a capture after it always falls inside a strike. The strike
lights the GI's sky light (above). Exarust draws Bevy's atmosphere for the
open sky (see below) and keeps the gradient for the record and the fog. The
frame also publishes the values the renderer's globals take: a scene value
from the sun's height (−100000 up to 0.4, −25 above it, compared in double
precision; probably a shadow or clip distance, its reader is not ported),
the weather wind (the current rain × 0.3) and the tint (the tint colour ×
its fourth slot, the strength).

**Ambient.** The plug-in's ambient is the level's main ambient through the
sRGB transfer, scaled by the environment cube at 8 indoors and 3 outdoors
(the shader's `Ambient.w`). The transfer's knee test and linear segment
are double precision; its power goes through the engine's fast logarithm
and exponent, for which Exarust uses `powf`. Exarust's shading takes the
cube path's ×16 stand-in when GI is off (see [Voxel GI](#voxel-gi)); the
level's ambient colour itself is reproduced, and the model exposes the
8/3 scale.

**The sun light.** `_Sunlight` is a directional light of sRGB 0.75 grey
(4.5 units under a sky, 3 without; 160 lux a unit in Exarust) taking the
day record's sun colour under a sky. Its node is created with the kind 2
light word, flags `0x20001000`, a 3 m range and the short direction vector
(0, 260, −280) before its matrix is built; `Lighting`'s sun carries the
range as data, since a Bevy directional light has none. In auto mode it
exists only in a scene with no lights and no grid rooms, and disappears
again when the scene has more than one room; the off and on modes remove
and create it.

**Weather.** Each frame the rain amount fades toward its target at
0.0001 a millisecond (in double precision; a NaN amount falls): a request
of 0.01 or less clears it, a request below 0.3 is raised to 0.3, and more
than 0.75 brings thunder. A thunderstorm schedules strikes
4 000…33 999 ms apart; a strike lasts 100…599 ms (its end and the next
strike's time are one word, so a rain request during a strike moves its
end), flickers its intensity every 50…89 ms by ±(0.1…0.3) scaled by a
distance attenuation (1 − distance·0.0002)², never below 0, and plays
thunder 30 ms plus 1.75 ms a centimetre after it starts (rounded half to
even) at a volume of attenuation·4 + 0.1. The time of day advances by the
speed once 16 ms have passed in a frame. The original's random numbers are
the wrapping linear congruential generator 134 775 813·state + 1, drawn as
state/2³², except a strike's first step (0.2…0.3) and the thunder's pitch
(0.9…1.2), which come from the run-time library's own generator. Exarust
reproduces the rules and their generator; the outdoor scene preset is the
one thing that starts rain and redraws the time of day, at a level load.

**The presets at a load.** The save unit keeps the scene presets that the
arena's scene creation calls: the indoor/underground preset (the sun off, no
sky, a 50 000 cm far plane, fog distance 2, the full-strength grey global
light, a black background, 0.5 grey fog, no wind or rain) and its 1.4, 1.8
and 2.0 fog variants; and the outdoor preset (a 100 000 cm far plane, fog
distance 0.6, the grey 0.6 global light, the sun on, a sky, then a seeded
draw of the time of day, the rain, the fog distance, the cloudiness and the
wind). The draw uses a local stream started at the caller's seed, replaced
by a clock seed when zero; three of seven rolls rain, and only an arena
whose entry in the original's table allows it may draw a night (0.76 … 1.0
instead of the eased 0.32 … 0.74 day). A level load now applies these
presets: `exa_world::environment::level_preset` classifies the level from its
name (`arenao*` outdoor, `exanima*` and the first seven arenas indoor,
everything else by its record), the first four outdoor arenas allow the
night, and `level_lighting_preset` runs the preset before deriving the
`Lighting`. The indoor preset leaves an indoor level's picture as it was
(those levels had neither sun nor fog); an outdoor arena now draws its day
and weather per launch, as the original does. The renderer's view also takes
the preset's sky/sun value: zero on a day, and on a night `0.5·(cloudiness
− 2) + (the previous time of day − 0.7)` in the original's mixed precision
when the caller's view flag is set (the arena's set-up always sets it).
The renderer's view reset and the audio environment switch the preset also
performs are the caller's.

**Grid levels.** A tile-grid level built the original's older way forces a
black background and 0.5 grey fog, and, unless the level has terrain, an
ambient of 0.6275 grey and a global light of 0.75. Exarust keeps that as
`SceneEnvironment::force_indoor`; it builds grid levels the newer way, so
the game does not apply it. A floor set named exactly `Terrain` is not
loaded (the terrain is the ground), and a special tile's mask turns with
the cell's rotation; see [formats/RFC.md](formats/RFC.md) (Grid building).

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
  values q, p. Node flag `0x40000` with a radius above zero makes the light
  fall off with distance. Without both, it reaches everywhere at full
  strength: the original's activation then leaves the attenuation 0 and
  the far plane unbounded.
- Lights with spot values are shaped vertically:
  - y = ½·sin(qπ/2) for q ≥ 0, otherwise 3 − 3/cos(qπ/2); z = p + 1;
  - factor max((2^(z·(Ly + (1 − Ly)·y)) − 1)/(2^z − 1), 0), with Ly the
    vertical part of the direction to the light (1 straight below it);
  - applied on surfaces and in the GI injection.
  
  Exarust keeps y and z in a per-light record the shaders read
  (`exa_render::lights`). Each light's id travels above Bevy's default
  depth bias (0.08) in its depth-bias field, in steps of 1e-5: the shader
  recovers it as round((bias − 0.08)/1e-5). The field stays a valid bias
  in case Bevy's own point shadow maps are ever sampled; they are off.
  Exarust sets Bevy's range to 1.25·R and computes (1 − d/range)⁴ itself.
  Point lights also keep the original's 3 cm shadow-map near plane, not
  Bevy's 10 cm default: at 10 cm a torch's close-range character shadow
  smeared over the floor instead of following the body.
- Lights cast shadows from cube maps (512² per face; the sun's map is 4096²
  at the top shadow detail). The allocator picks the class by the light's
  kind (2 directional, else cube) and the size from the shadow detail (a
  zero detail takes the user's setting, counted as 5 above 4) plus the
  light's quality offset. Nothing clamps the sum: an index of 0 gives no
  map, one outside 1–6 still gets one at the fallback size. Directional
  sizes are 256/512/1024/2048/4096/8192 at indices 1–6 (2048 for an index
  outside the table); cube sizes are 64/128/256/512/1024/2048 (512 outside),
  halved, shifted right by the light's reduction level and, for both, capped
  by the hardware maximum as signed numbers. A cube map's own draw routine
  sizes it again when it first renders: the user's detail plus the light's
  offset clamped to index 6 (or, when the renderer has its own detail, the
  light's offset plus that detail with no clamp), the cube size halved, with
  no shift and no cap. It also stores `0.005·index` (in double precision;
  0.025 above index 5) on the light for the lighting pass, a flag for a user
  detail above 4, and takes its map from a pool of equally sized maps: the
  first of its size without an owner or whose owner is not in use, else a new
  one; a shadow-detail change drops every map. The caster programs are the
  plain depth pass, the distance writer, each of those two with an alpha
  test, and a distance writer with a dithered fade (`fract(x·3.555713358 +
  y·0.309269245)` against the caster's fade value, only without an alpha
  test); the cube pass picks the fade program for any caster whose fade
  flag is set. Casters with the front-cull flag draw with front faces
  culled and the others with back faces culled. The reference-counted
  shared cube texture pool exists in the original but its map constructor
  sets the flag that bypasses it. The directional view matrix normalises
  with the processor's reciprocal square root estimate and sums its
  translation as `z + (y + x)`.
- Surfaces with opacity ≤ 0.8 in double precision (glass, decals) and those
  whose record has extra flag `0x4` cast no shadows; the compare widens the
  stored f32 to f64, so `0.8` stored as f32 is above the bound. A record in
  refraction mode (`0x48 & 0x300` = `0x200`) always casts, whatever its
  opacity; a record whose loaded blend-word destination nibble is ONE never casts
  (an alpha-tested word has become destination ZERO, so it can).
  The others cast whole, except through
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
  cached cube maps (every caster of point lights); Bevy's own shadow pass
  for the sun still cuts as the colour pass does (`exa_prepass.wesl`).
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

  Links are kept current incrementally: a moved mesh is relinked against
  every light (a character's skinned mesh once it has moved 10 cm), and a
  moved light against the meshes that can reach it, as the original
  relinks a light against the nodes of the sectors it reaches: the meshes
  filed in the 6 m cells within its radius plus a cell (by their sphere's
  centre), the meshes wider than a cell, and those already linked to it. Each light keeps its linked meshes as its cube's
  casters, each with the cube faces its bounding sphere touches, as the
  original keeps a face mask per link: a face is left out only when the
  sphere lies wholly beyond one of the face's four side planes, through the
  light at 45° between its axis and its neighbours' (farther than its
  radius; exactly its radius still touches). Each light also has a render layer of
  its own (from 2 up; layer 1 is the supersampling resolve quad) as the key
  of its links; every mesh carries layer 0.

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
  the test. The GI takes its lights from the same cell lists, sector by
  sector (see [Voxel GI](#voxel-gi)).

- **Cached cube shadows** (`exa_render::cube_shadows`): every caster,
  characters included, casts into cube maps kept across frames, as the
  original keeps its shadow maps:
  - a face is rendered only when it is needed (its pyramid out to R meets
    the camera's view) and dirty;
  - every face goes out of date when the light moves or takes a slot. A
    caster linked or unlinked dirties the faces its sphere touches; a
    caster that moves dirties the faces it touched and the faces it
    touches now, as the original ORs a moved node's old and new masks into
    its light. An animated character dirties the faces it stands in every
    frame; the light's other faces stay cached;
  - a face draws only the casters whose sphere touches it. A caster's faces
    are worked out again when it moves and, for all of a light's casters,
    when the light moves, as the original refreshes a moved light's links:
    a carried torch's casters otherwise kept the faces of where the torch
    had been, and a caster that crossed into another face was left out of
    it, its shadow cut at the face's edge;
  - characters' skinned meshes are posed in the caster shader: each vertex
    is the weighted sum of its joints' world transforms times their inverse
    bind poses, computed once a frame for a character drawn into any face;
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

  Bevy's own point-light shadow maps stay off. They re-rendered all six
  faces of every light with a character linked, every frame: in exanima01's
  play mode about 111 extra views, 113 runs of the camera schedule and 743
  command-buffer submissions a frame, the render thread's 29 ms. With the
  characters in the cached cubes the render thread takes about 7.5 ms.
  Skinned meshes carry per-joint boxes (`DynamicSkinnedMeshBounds`); their
  spheres give their faces.

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
    floors, 1 for other grid set pieces, and otherwise the level its mesh
    ships with (bits 16–18 of the mesh's render flags, [formats/RFC.md](formats/RFC.md);
    3 317 shipped meshes carry one, the dungeon's broken floors
    `uw01_floorbrk*` level 2). For v > 0 the original divides receiver
    distances by k_r = 1 − 0.005·v and caster distances by
    k_c = k_r − 0.01, so a surface keeps about 1.5 % of the light distance
    between itself and its own shadow. That is what keeps floors free of
    acne at grazing light. The cached cubes apply both sides, cross terms
    included. The cross terms matter: a level-3 floor
    receiving from a level-0 caster is shadowed by surfaces up to about 1 %
    of the light distance *behind* it. Read as level 0, the broken-floor
    slabs, which lie 1–2 cm under the painted dirt, darkened the ground over
    them in soft bands (exanima01's start room); at their shipped level 2 a
    caster must be about 1 % in front. The variant with bit 19 (both sides
    × (1 + 0.005·v), one mesh, `Pave03Brk`) is not reproduced: it keeps
    level 0;
  - in a supersampled frame each sub-frame takes two of the eight taps
    (sub-frame i takes taps 2i and 2i + 1) and the turn is per window
    pixel, so each output pixel still averages all eight. Exarust's
    sub-frames are the four pixels of each 2 × 2 block of the scene image.
    The GI lookup splits its taps the same way.
- The sun's colour per unit of intensity is 160 lux in Exarust's scene units
  (scaled back in the shader).
- **exanima01's start room.** Two level lights hang over the spawn:
  - the pool light, 4 m up: grey (sRGB about 0.81), intensity 60, radius
    630 cm, GI multiplier 0.1, shaped straight down (q 0, p 15: shaping
    (0, 16));
  - a faint fill, 2.5 m up: white, intensity 0.1, radius 2450 cm, GI
    multiplier 0.

  On the floor 3.9 m below, the pool's shaping halves about 1.5 m out and
  falls under a tenth at 3 m. That is the cold white pool, about 3 m
  wide, with white specular, that the original shows around a player who
  holds no torch. It is not the player light, which a normal start leaves
  off (see `docs/PLAYER.md`, The player light). Exarust draws both lights.
  Its exanima01 start puts the torch in the player's hand, and the torch
  drowns the pool; the original's story start has the torch lying on the
  floor beside the player.

  Matched to a retail capture (1600×1080, the retail FOV 29.93, torch laid
  on the floor with `--floor-torch`), the frame measures (25.5, 16.9, 11.4)
  sRGB against retail's (24.2, 17.0, 12.2), and the pool 70.9 against
  70.6. Retail's distant surfaces change from frame to frame: the top wall
  reads 9.0, 6.3 and 2.9 in captures 2 s apart, and Exarust's reads 5.8.
  Not explained yet: in the dim periphery, retail's neutral light is about
  1.7 times Exarust's. Exarust lights it only with the fill's direct term,
  and its GI adds nothing there.

### Sun shadow fit

The original's sun has one orthographic shadow map (no cascades). The map is
`{256, 512, 1024, 2048, 4096, 8192}[q − 1]` texels with
`q = min(ShadowDetail, 5) + the sun's RFC flags bits 0–1` (0, +1, −1, −2; 0
means no map), DEPTH_COMPONENT16, clamp-to-edge, linear, compare to texture,
less. The sun's own flags give 0, so the top detail is 4096²
(`exa_render::SUN_SHADOW_SIZE`).

The original fits the map only when the scene has a terrain. Every other level
covers the whole scene instead: the scene box's centre rotated into the light's
frame, the box's radius (the length of its half-extent vector) as the half
extent and as the up-sun reach, the scene's radius down-sun, each raised to
the scene's minimum extent of 1 000 cm, and no texel snap and no hysteresis.
The half extent's raise is `min > h` (a NaN never raises); the down-sun
reach goes through the engine's `maxss` helper, so a NaN radius gives the
minimum.
Its softness is `(c + 1)·(200 / h)`, without the fitted mode's 0.15 floor.
Exarust takes the radius as the box's half-diagonal and the down-sun reach as
the same value (the original's own radius comes from the scene-node unit, out
of the component's reach). Every frame of the fitted mode the update copies the
camera's
view volume (the camera view's frustum: six quads; the constructor's camera
has a 100 cm near and a 125 m far plane, what the game sets later was not
traced), clips it by up to three planes, fits the smallest sphere around the
surviving corners, and works in centimetres (×100):

- **Clipping.** The planes follow the camera, not the sun: per axis, the
  sign of the camera's *backward* axis (its frame's third row) picks the
  plane. A negative component (the camera looks towards +axis) keeps
  `axis ≤ high`, a positive one `axis ≥ low`, a zero or NaN one adds none,
  so the view is cut where it leaves the scene. On x and z, high and low
  are the scene box's centre ± half extent. On y they are the level's
  ceiling and floor from its environment record (whole centimetres); a
  level without the record takes the box's *x* centre ± half extent there,
  as the original reads it. The update also writes 150 cm and 650 cm into
  the box's vertical centre and extent first; the fit never reads them.
  Another camera mode (not in Exarust) clips both vertical planes instead.
  A face keeps the part where `normal · p − w ≤ 0` (a NaN is outside); the
  cut is the engine's convex-volume clip (`exa_physics::intersect`).
- **Corners.** Every face's vertices, in face order, merged within 0.1 cm²
  (squared distance summed `(x² + y²) + z²`, a NaN never merges). Fewer
  than two corners end the whole update: the map and its matrices stay as
  they were. The corners are then scaled by 0.01 into the solver's metres
  and the fitted centre and radius are scaled back by 100.
- **Radius.** `h = 200·round(r/200 + 0.4)`: the quotient is single
  precision, widened before the 0.4 (double) is added, and converted with
  the processor's default rounding (nearest, ties to even; a NaN or an
  out-of-range value gives the lowest 64-bit integer). The half extent is
  kept across frames: the quantised value is taken only when it differs
  *and* `r > h + 200·0.3` (grow) or `r < h − 200·0.1` (shrink), both
  compared in double precision. The constructor starts it at 3000 cm. This
  mode has no minimum extent: the scene's minimum (1 000 cm in a level: the
  scene's constructor multiplies that by a global factor of 1; other scene
  kinds store 100 cm) is read only by the whole-scene mode.
- **Centre and depth.** The centre is rotated into the light's frame (its
  travel direction is the negated, normalised vector towards the sun, the
  length summed `(x² + z²) + y²`; `right = dir × +Y`, `up = right × dir`;
  past |dir.y| > 0.9999, compared in double precision, the fixed frame
  right +X, up −Z, forward −Y whichever way the light points). Its x and y
  are snapped to the texel grid (`round(c / texel)·texel`, `texel =
  2h / size`, single precision, the same conversion) and the box extends
  ±8000 cm along the light from it.
- **Softness and bias.** The same update writes the sun light's softness
  `(c + 1)·(400 / h)` (c the light's own value, 0 by default); below 0.15
  (a double compare) it writes 0.15 and `c' = 0.15 / (the unfloored value)`
  in double precision. The render routine's polygon offset is
  `1 + 1000/h + 2000c'/h` with one offset unit, and it nudges the casters
  (not the receivers) by `0.01·w`, `w = ½·min((c' + 1)·500/D, 0.3)` with `D`
  the box's near + far reach and the 0.3 compared in double precision: in
  the engine's `[−1, 1]` depth that is 1.5625e-4 at the constructor's
  16 000 cm and `c' = 0`. Exarust writes the nudge into Bevy's drawing
  matrix only, halved into its `[0, 1]` reverse-Z range, so receivers keep
  the unbiased lookup; the polygon offset has no Bevy counterpart on the
  caster side and stays a returned value.

The render routine's caster pass walks the scene's caster list: a node
without the caster flag is skipped, a flagged node clears its map-visible
bit, the list's head is kept without the volume test (and without moving the
volume), every later node is skipped when its box lies wholly beyond a face
of the caster volume (its centre farther than the box's radius, or a positive
distance beyond its support along the face normal, a NaN never culling), a
kept later node's stored vector pushes the volume's faces in place by its
projection (for its own parts and every node after it), a kept node sets its
bit, and the node's parts are tested the same way: an unflagged part keeps
its map-visible bit, an outside part clears it and draws nothing, an inside
part sets it, switches to the cull face its flag picks (front or back,
through the cached switch) and draws each of its meshes whose draw flag is
set, plainly or, for a mesh marked alpha-cut, through the alpha-tested
program with the mesh's texture bound and another geometry state (taken to
be culling off; the state word is not decoded). Exarust models the walk
in `exa_render::sun_casters` and pins it with tests; nothing drives it yet
(Bevy's shadow pass draws the cascade's casters with its own frustum culling,
and Exarust has no scene-node caster flag or per-part box at the original's
fields). The caster volume itself is built out of the component by the
intersection unit (the light-facing faces of the view volume plus side
planes extruded along the sun through the silhouette edges, vertices merged
within 5 cm² and edges shorter than 1 cm² skipped); that builder is not
ported.

The solver is `exa_render::bound_sphere`, reproduced to the bit (reverse
engineered from Exanima.exe):

- The points are widened to double precision; the result is narrowed back
  to single (centre per component, radius = √r² rounded once).
- Move-to-front: the sphere starts at the first point, radius 0, resting on
  it. The scan visits the points in order and skips those in the support
  set or within 1e-5 (squared distance minus r²). For any other point q
  the support set and q give a new sphere; if its r² is larger the sphere
  is replaced and the scan restarts at the first point. The support set
  keeps what the step made of it even when the sphere is not replaced.
- Candidates, for a support set of k (1 to 4): the spheres through q and
  one, two or three support points, in fixed orders (k = 2: 2 pairs and the
  triangle; k = 3: 3 pairs, 3 triangles and the tetrahedron; k = 4: 4
  pairs, 6 triangles and 4 tetrahedra). One whose r² is below the best so
  far wins when the remaining support points lie inside (with the 1e-5
  slack); otherwise the one that misses by least is kept as the fallback.
  The support set becomes the winner's points, in its order, then q. With
  k = 2 the winner's slot order differs: q replaces the second point, the
  first, or joins as the third. With k = 1 the sphere is always the pair.
- A pair: the midpoint, r² = d²/4. Three points: the circumcentre
  a + s·(b − a) + t·(c − a) in their plane, or an edge sphere when it falls
  outside (s ≤ 0: a–c, t ≤ 0: a–b, s + t ≥ 1: b–c); a flat triangle
  (|2·det| ≤ 1e-7) gets r² = 3.4e38 (the single-precision constant), so it
  never wins. Four points: barycentric weights from the Gram matrix of the
  edges from the fourth, solved by its adjugate; |det| ≤ 1e-7 gives
  centre 0 and r² 3.4e38.
- No points: only the radius is written, 4000 in the points' units (the
  shadow fit never calls it with fewer than two).

Exarust's fit is `exa_render::sun_fit`: plain functions for the planes,
the corners, the sphere (`bound_sphere`), the quantisation, the snap and the
matrices, plus the `SunFit` state that carries the half extent between
frames. In a level with a terrain, `exa_world::sun_shadow` builds each active
camera's frustum out to 125 m (the original camera's default far plane),
clips it at the level's box (the props' origins and the terrain's tiles) and
the record's floor and ceiling, and replaces the sun's Bevy cascades for that
camera with one cascade holding the fitted matrices, each frame after Bevy's
cascade builder and before it derives the shadow frusta (so the CPU caster
culling follows the map too). The sun's `CascadeShadowConfig` then has one
bound at 125 m. A frame that fits nothing keeps the previous matrices.
Levels without a terrain get the whole-scene map the same way (one cascade per
active camera, independent of the camera's view, with the bound set to the
scene's diameter so that a camera anywhere in the scene keeps sampling it). Every level with `--bevy-sun-cascades` keeps Bevy's two
cascades over 60 m. Both
use the 4096² map.

What the fitted map looks like: one texel is `2h/4096`, a few centimetres
for a view the scene bounds to a few tens of metres; the centre snap keeps
texels still while the camera moves and the hysteresis keeps the size still
until the view grows or shrinks by a fraction of a step, so the edges do not
swim. Bevy's normal bias scales with the cascade's texel size, which the
fit reports, and its depth bias is in metres along the light, so the bias
holds as for its own cascades: no new acne or peter-panning. The depth range
is 160 m in a 32-bit float map. Casters up-sun of the box's near plane are
still drawn (Bevy culls directional casters without the near plane and
clamps their depth). Receivers outside the fitted box are lit, where the
original's clamp-to-edge map repeats its border texel.

Not reproduced: the softness upload, which needs a filter-radius uniform
Bevy's directional shadows only have with its PCSS feature (the value is
computed); the polygon offset (Bevy's own depth and normal biases apply);
the other camera mode; the caster-volume builder; the map's own GL lifecycle
(Bevy allocates a 32-bit float depth map where the original asks for 16-bit,
and the original's optional RGB8 colour twin behind a debug switch that
ships off). The scene box is a
stand-in: the original's comes from the scene's own bounds, Exarust's from
prop origins and terrain tiles. Exarust's record ceiling falls back to the
grid top when the record's highest point is 0, where the original would clip
at 0 (whether a shipped level has a zero there was not checked).

## Fires

The original's fire presets turn a fire marker into particle emitters and
the light they drive. Exarust reproduces the templates, the tier choice,
the emitters and their rules in `exa_world::fire`, `exa_world::flame` and
`exa_world::particles`.

**Tiers** (`CreateFire`, size s; the stored size is a float, e.g. a torch's
0.2 read unchanged from the model's `_Fire` node):
- `s ≤ 0.35`: the small template (`_FirePrt02`), one emitter at the node.
  The light is 8√s, radius 1500√s cm, ×1.25 when s > 0.2 (compared in
  double precision, so a torch's f32 0.2 widens): 3.58 and 838.5 cm.
  Burning items use this too (0.2 from the marker, or 0.25–10 from an
  object record's percent).
- `0.35 < s < 0.5`: the medium-small flame (`_FirePrt01`, 8/1500) and a
  `_FireBas` bed 15s cm below it; the light sits 40 cm up.
- `0.5 ≤ s ≤ 1`: the medium flame (12/1600) with a bed 20s cm below,
  scaled by the whole size (the 0.35–0.5 bed by half of it); the light
  50 cm up.
- `s > 1`: the large flame at half the size (12/1500) with a bed 40·(s/2)
  cm below; the light 40 cm up. Both emitters are one-shot: they stop when
  their ring wraps and, once it empties, the fire is removed.
- The flame gets `round(√s · 4)` (or 5) sources on a ring at 20–40 s cm,
  each at least π/n apart in angle; the beds have one source at the node.

**Templates.** Every field is the original's: rate (250/40/100/80/80/70/100
per second), lifetime (1.25/0.7/0.5/0.65/0.5/0.6/0.5 s), half-size
(10/18/16/30/25/50/25 cm), speed, retention (0/0/0.5/0/0.5/1/0.5 per
second), growth, stretch (0.6/0/0/0.75/0/0.6/0), ring (35/15/100/30/45/45/50
cm), buoyancy (−1.25/−0.5/−0.05/−1.5/−0.33/−1.5/−0.33 × gravity), centring
(0.0007/0.0005/0.001/0.0002/0/0.0005/0), flags, light, sound
(`FireS02`/`FireS01`/`FireS02`/`FireL01`) and the colour keys. The size
scales half-size by 0.5 + 0.5s, speed by s², volume by s, rate by s
(rounded to even), ring by 0.5s + s², buoyancy by s, the light by √s, and
the density normaliser by 0.9s² + 0.1s. The keys' RGB are scaled by 0.7 at
load (K1…Kn, the original's table starting with its count word:
byte·(1/255)·0.7·255, rounded half to even, so 160 becomes 112; alpha
untouched; the 0.35–0.5 bed's table is not scaled). The legacy non-linear
path (which the shipped pipeline flag never selects) encodes each channel
to sRGB (linear up to 0.0031308, the engine's fast power above), pulls it
0.35 of the way toward its Rec. 601 luma (0.299/0.587/0.114), scales 0.75
and stores three saturated bytes (`load_scale_legacy`). The beds'
keys carry no alpha, so they add their colour without dimming; they peak
two thirds (0.35–0.5) and half way (the others) through a particle's life.

**Emitter state and order** (all at the 83 Hz tick, one ring of
`capacity + 1` slots):
- Derived: interval = (83/rate)·⅔ (in double); lifetime = trunc(83·life);
  per-tick speed = speed/83; per-tick retention = exp(ln v / 83) in double
  (1 when v ≤ 0); growth = template · half-size; pool =
  sources·trunc(rate·life·1.5 + 1.501) + 2 (a torch: 97); density
  normaliser = trunc(rate·life) · sources.
- Random draws: integers from the engine's linear congruential sequence
  (`seed·0x08088405 + 1`, `(seed·n) >> 32`), units from the run-time
  library's Mersenne twister (×2⁻³², double). Both are process-wide and
  clock-seeded in the original; Exarust keeps one pair per emitter.
- The emitter enters the world at its node: its sources' last positions
  and its measured motion start there, so the first particle is not flung
  from the origin.
- Each tick, in order: the light's flicker walk and density; ageing (dead
  at the head popped, extra losses, the distance cap); the burst refund;
  Verlet integration; the one-shot/finite checks; the accumulator's
  two-thirds loss; spawning while the accumulator is below one; the
  bounding box (centre = min + (max − min)/2, extent + 5 cm). The head pops
  count one more when they empty the ring onto a dead slot, which the burst
  refund (−(min(k, 4) − 1)·interval for k > 1) sees.
- Ageing: with the fire flag every particle loses one life a tick, half
  again at random, and its life is capped to round((R − D)·L/R) when
  shorter, where R = 140·(the emitter's size), D = |(x, 0.7y, z)| from
  the node (0.7 in double) and the round is half to even. A torch dies
  after roughly 30–60 ticks, 40–55 cm up.
- Integration: d = p − q; q = p; d.y += gravity·buoyancy (gravity
  −980.665·(1/83²) cm/tick²); p += d·retention; then p.x/z move a fraction
  `centring` toward the node's x/z. There is no other drag or noise.
- Spawning: at the source frame, plus a ring offset in world XZ,
  r = R_ring·(1 − u⁴) (u uniform, u⁴ in double); mode 1 draws a random
  direction x/z = spread·(rand(2001) − 1000)/1000 and y = rand(1001)/1000
  (in double), x·axis₀ + y·axis₁ + z·axis₂ of the source frame (a ring
  source's second axis points out of the ring); other modes use +Y. The
  pair is q = p − c·dir, p += c·dir with c = the per-tick speed, which is
  zero for every small fire: a torch's particles start at rest. The
  previous position then loses 0.3 of the source's own motion. Draws:
  lifetime (when randomised), UV variant, ring angle and u, then the
  velocity's three integers, in that order.
- A carried torch's fire rides the item's body [inf]: each tick its live
  particles move by half the body's velocity at the emitter (here the
  node's displacement over the tick) and new ones inherit 0.3 of it, so
  the flame trails a little behind the hand. An emitter attached to a body
  (`Emitter::attach_body`) takes the body's whole frame for its sources and
  particles and turns its fire ageing on, as the original's body-attached
  emitters do.
- A fire marker on a model's geometry suppresses the host mesh's own
  emissive at import (its first part's material gains the off flag and both
  emissive scales go to zero) and becomes a fire only when the scene lets
  fires run; a re-created fire keeps the host node's draw bits
  (`0xc01f000`).
- The light (all in double): x walks softly in ±1 with v ±= u past the
  bounds and else v += 2(u − ½), x += 0.3·v·u (three draws); jitter +=
  (u − ½)·0.125 in [0.87, 1.15]; density = 0.8·density +
  0.2·live/normaliser. A new peak needs a strictly brighter frame.
  Intensity = density·(1 + 0.05x)·base; softness = 2·jitter; an emissive
  parent's scale is 0.8·density·(0.9 + 0.2x). Sound: with the motion flag
  the volume follows the smoothed emitter motion; otherwise it walks
  ±20 % of the template's volume; the pitch walks ±0.005 (or ±0.05) in
  [0.85, 1.15] (or [0.6, 1.4]). Exarust keeps the walks but plays no loop.

**Drawing** (per frame, oldest first, no depth sort):
- Position p = q + (p − q)·α; age = 1 − (life − α)·(1/L).
- Mode 4 (flames): w = (p − q) − (0, buoyancy, 0); d = −w/|w|; A is the
  unit vector from the camera to the emitter's centre (both in cm);
  c = d × A, ℓ = |c|; k = S/ℓ + age²·growth; X = c·k (half-width
  S + ℓ·age²·growth); Y = c × A; stretch = (2 − 2/(|w| + 1))·stretch·
  acos(|d·A|) with the original's fast arc cosine
  (π/2 − 0.2075x + 0.0535x²)·√(1 − x); corners P − Y·(1 + stretch)·k,
  P + X, P − X, P + Y·(1 + stretch)·k (tail, sides, head). Lengths add
  x² + z² first, then y².
- Mode 1 (beds): a quad facing the line to the emitter's centre about the
  camera's up axis, half-width S + age²·growth and half-length
  (1 + Y.y)·that, Y the quad's unit up axis (both axes the normalised
  cross products with the line to the centre, by `RSQRTSS`). Mode 0 is a
  flat billboard of half-size S; any other quad mode draws nothing.
- The screen and stretched quads keep the camera's own axes, or express
  them in the node's frame (each axis dotted with the node's three axes)
  when the template sets flag 0x40; a template with flag 0x1000 keeps a
  tenth of each axis' component along the node's up axis (the dot times
  −0.9 in double) and renormalises, flattening the quad onto the node's
  horizontal plane. No fire template sets either flag. The fixed-quad path a material can ask
  for (its flag bit 2) is the four camera corners
  (0.3 forward ± 0.5 of the right/up axes, normalised), the same for every
  particle; no Exarust material asks for it.
- UVs: one of the four atlas cells × one of the eight square symmetries
  (variant 0..31), or a direct 8-symmetry variant.
- Colour: the keys at f = n·age, K[k+1]→K[k+2], each channel rounded half
  to even and saturated; odd ring slots start one key later; flat mode
  uses a = round((1 − f²)·255) and the material's opacity.
- A hidden flame keeps ticking but builds no quads (the original builds
  them only for a visible node).
- Output: texture × vertex colour, premultiplied (ONE,
  ONE_MINUS_SRC_ALPHA), no depth writes. The beds are additive in effect
  because their key alpha is zero.
- The two particle textures load through the texture manager's streaming
  path (the original's asynchronous load): an emitter binds the 1×1
  placeholder until the loader's upload arrives, and the flame material's
  bind group is prepared again when the image under its handle changes.

**Per-frame values.** The light's and sound's walks run once a rendered
frame, as the original's per-frame pass does, not once a tick. The
intensity and the emissive scale are worked out in double precision
(intensity = density·(1 + 0.05x)·base, emissive = 0.8·density·(0.9 + 0.2x));
the first frame, a one-shot template's every frame and any frame brighter
than the peak ask for a full light update, and a new peak is 1.1× the
intensity; the emitter's first-frame bit clears after every frame and a
separate at-rest bit once the intensity reaches the template's base. The emissive scale reaches `Fire::emissive` and, through
`exa_world::fire`'s host system, the parent mesh's first part (the
original writes it into the parent node's own scale and its first
sub-object's material, and into the same pair of each of its geometry
children, which Exarust does not reach yet; the part gets a material copy, and its loaded emissive colour is
the base the scale multiplies). Each emitter also
keeps a *detail* value apart from its size: the material's level of
detail is `detail·1000/d²`, d² the squared distance from the camera to the
node held at one or more (the original's distance helper returns the
squared length; `Emitter::lod_scale`, not yet bound to a material). The small
and medium flames' detail is the fire's, the medium-small and large
flames' twice it, and every bed's twice its own size. A one-shot template
records its ring deadline (start life + the ring's span + ½, rounded half
to even), which the original stores for the game to read.

The template's looping sound (`FireS02`, `FireS01`, `FireL01`) plays from
`exa_game::fire_sound`: one looping voice a flame on a handle of its own,
loaded once through the sound bank by name, moved to the flame's node and
given the emitter's walked volume and pitch every frame, and freed with its
buffer reference when the flame is despawned. A template with no sound, no
game data or no audio device plays nothing, and a sound that fails to load
is not asked for again.

Still missing for fires: fires on other physics bodies (only the carried
torch rides its item), body contacts being driven from the physics step,
the particle material's parameter block (its 0.004 tint) and
the node bookkeeping the scene hierarchy replaces (the emitter's per-frame
node transform copies and its child lights' transform copy). The
quality-scaled pool override is dead in the original: its branch needs the
quality constant to be below 4 and the shipped constant is 5, so the
template's pool is always used.

## Electricity

The original's electric arcs (`RFE_Effects_Electricity`) are reproduced as
rules in `exa_render::electricity` and a Bevy layer in `exa_world::arc`.

- **Presets.** An electrified weapon: 20 tendrils of 5 segments, size 2 cm
  (width 4 cm; segments of 0.75·8 = 6 cm), sustained, its tendrils renewed
  every 10–19 ticks (the run-time library's Random(10) + 10), jitter 0.075,
  reach 24 cm, the `electricity03` hum and the voxel light (0.6, 0.6, 1)
  of 16 at 180 cm, 53 cm up its item. A spark burst: 20 + 6 tendrils of 5
  segments at size 3 (width 6 cm, segments of 2·12 = 24 cm), one-shot over
  10–19 ticks, jitter 0.05, reach 8 cm, the voxel light (0.5, 0.5, 1) of 8
  at 100 cm. A level device from its object record: size = duration/1000,
  segments = round(length/(4·size)) (10 when below 2), tendrils = branches
  (10 when 0), jitter 0.1, a second tendril value of 0.5, reach size/2,
  the `electricity01` hum, and a point light (0.6, 0.65, 1) of size/2,
  multiplier 2, softness 5, radius 80·size cm; no voxel light. The record's
  fields (`DeviceFields`) are its active bit (bit 0 at 0x0C, no arc
  without it), duration (0x10), length (0x12), branches (0x14) and flag
  byte (0x0B), parsed totally. The first segment runs along the node's
  third axis; flag bit 0x02 adds a fixed end point the record's length
  along it (`DeviceFields::end_point`). The geometry is sized as the
  original allocates it: (4·segments + 4) vertices and (4·segments + 2)·3
  indices a tendril record (twenty for a spark burst, whose six extra
  tendrils take free records), every vertex coloured `0xff0019`.
- **Record.** The 84-byte arc record holds flags, tendril count, segments,
  the tendrils' lifetime in ticks, size, the segment length's scale, a
  reserved word, a second tendril value (0x1C), the reach (0x20), a
  15-byte sound name (none when empty), the jitter (0x34, 0.1 when not
  positive), an offset, a direction and a drift scale (0x50). Flag 1 makes
  the arc sustained, renewing every n + Random(n) ticks; without it the arc
  is one-shot over n ticks, its first segment along the direction. Flag
  0x200 adds the weapon's voxel light; 0x300 registers an electric flow
  with the voxel GI instead. A short record is an error; nothing panics.
- **Charge.** An electrified item adds 10 a hit (0x80 when the part kind
  has bit 0x10); the step that would reach 255 fills the charge and
  releases the effect. With a live part the arc is created unless a live
  one exists; a dead part releases it. Releasing stops the first arc child
  renewing, sets its state to 1, and clears the owner's callbacks. The
  equipment check scans the body's two hand slots for the item, needs its
  object record, and then runs that release or creation
  (`electrified_check`). A hit queues a discharge at a point: it steps the
  charge (heavy when the record's kind word has 0x10; a spent charge or a
  missing record clears the item's arc effect) and always spawns the spark
  burst there, charged or not (`discharge`).
- **Level tone.** The template selection's per-call values are in
  `electric_tone`: the effect level (a negative level is zero for the
  arithmetic; the template choice reads it as passed), the brief choice
  (`brief_effect`: level 5 never, below 3 always, 3 when the engine
  generator's draw below 2 is 0, above 3 when its draw below 8 is 0), the
  strength `(level·0.05 + scale) + (1 − level·0.05)·curve` with the level
  term in double and the sum clamped above at one (a not-a-number kept),
  the opacity `0.7 − strength·0.6` blended with a positive owner alpha as
  `alpha·0.3 + owner·0.7` (double) and, for the peak tone (`peak_tone`:
  level 5 with the owner's state word zero, never brief), redrawn as
  `0.4 − curve·0.375` from a second curve sample (product in single,
  difference in double), the intensity `roll + level·0.4` in double, and
  the template lifetime 0x16 (brief) or 0x44. Nothing calls the tone yet.
- **Geometry.** Each tendril is a polyline of `segments` points between the
  arc's ends (along +X, the segments end to end, unless a record or a
  device sets the direction and length), jittered inside them; each
  segment becomes a camera-facing quad of width 2·size, U along it and V
  across it. The original builds a strip of 4·segments + 2 vertices per
  tendril; Exarust builds four vertices a segment, a model, and pins the
  original's buffer sizes in `ribbon_capacity`. A spark burst's six extra
  tendrils start at the source in a random direction (sway 0), not at
  spread points; the original makes them one segment long and grows them
  (`exa_game::thaum::arcs::spark_burst`), this ribbon lays them out full
  length. The segment count, width and timing are
  the original's; the tendril's own per-tick
  motion is a model (the original's tendril emitter is a separate
  component), so the zig-zag's exact shape is [guess].
- **Shader.** `_ElecMat01`: `tc = (uv + TexOfs)·0.25` with TexOfs =
  (0, the tendril emitter's scroll, which each arc advances by 0.015 a
  tick, dropping back by one once past 2; its own material carries the
  offset); a = the
  texture's red mixed a third
  into the ribbon's V (across it); a = 1 − |2a − 1|; b = a²⁴; a = a⁶;
  colour = mix(vertex, EleCol (0.65, 0.4, 1), b)·b +
  EdgCol (0.15, 0.2, 0.4)·a·2; then squared. Additive, two-sided, unlit.
  The material's emissive texture is a white stand-in until its library
  texture is available.

Still missing: the exact per-tick tendril integration (jitter and segment
lifetimes; the live tendril emitter is modelled in `exa_game::thaum::arcs`,
which the Shock power and shock weapons use), the `_ElecMat01` emissive
map, the draw's supersample-pass loop (one draw per pass with that pass's
viewport and its clock, `Arc::pass_clock`; Exarust's supersampled camera
draws the arc once), the GI flow, the device's hum, and the callers of this crate's
arc layer (nothing spawns one yet).

## Force effects

`exa_render::force_effects` holds the Force effects' programs, ported from
the shipped GLSL (`effectforce_f`, `effecttorrent_f`, `effectsmoke_f`,
`effectshell_v` / `effectshell_f`, `effectfrcstrk_f`);
`exa_game::thaum::effect_draw` submits them
([THAUMATURGY.md](THAUMATURGY.md), The shaped-force aura).

- **Ribbons** (`ForceRibbonMaterial`, `force_ribbon.wesl`): the blast, bolt
  and wave chunks (`effectforce_f`) and, with the `TORRENT` definition, the
  torrent's streams (`effecttorrent_f`). The vertex stage is Bevy's mesh
  stage, which is `effectforce_v` up to the colour scale: `vTex` is the
  first coordinate set, `vTx2` the second (`ATTRIBUTE_UV_1`), `vCol` the
  colour bytes over 255. `TexOfs` is the uniform, `EmiTex` the noise
  picture. `effectforce_f` never writes its alpha; it is taken as 0, so the
  blast ribbon only adds. With the `SMOKE` definition it is the
  displacement tendril's `effectsmoke_f`: three `EmiTex` reads, the smoke's
  vertical taper and the black, alpha-only output the premultiplied blend
  darkens the scene with.
- **Shell aura** (`ForceShellMaterial`, `force_shell.wesl`): its own vertex
  stage on Bevy's skinning moves each vertex out 2 cm along its skinned
  normal (the skinned copy's `position + 2·normal`); the fragment stage
  takes the world position and normal through `LclMat · scale(100)` into the
  shell's frame in cm and runs `effectshell_f` as shipped (three planar
  lookups blended by the squared normal, the eight `ColPt` ripples, the fade
  plane `FdPln`, the rim and the violet ramp). The viewer direction is
  normalised per fragment (the original interpolates a per-vertex unit
  vector). The uniform is 272 bytes: `LclMat` 0, `ViewPos` 64, `FdPln` 80,
  `ColPt` 96, `TexPos` 224, `ColourInt` 240, `TexScale` 244, `Fade` 248,
  the push 252, `Gradient` 256. A held body's or a weapon's aura copies the
  body's rigid meshes (each copy a child of its source mesh, moved out 2 cm
  along its normal) and `LclMat` takes the world into the body's first
  part's frame, the original's object space; the fade plane comes from the
  meshes' box fitted in that frame when the copies are made
  (`effect_draw::draw_rigid_auras`). With the `STRIKES` definition (a Force
  Strikes weapon aura) the fragment stage is `effectfrcstrk_f`: fixed 0.01
  coordinates instead of `TexScale`, four ripple points at 0.07, its own rim
  (`(t − 0.1 + b²·0.2)² + f` for `t = max(−va, 0)`, doubled by
  `ColourInt`), the `Gradient` dimming and no colour limit on the alpha.
- **Render state** comes from the material records the routines create:
  `_ForceMat01` and `_ForceAura01` carry blend word 0x163 (source ONE,
  destination ONE_MINUS_SRC_ALPHA, layer 6), Bevy's premultiplied blend,
  without depth writes; the force record's flags 0x480018 are two-sided,
  the aura's 0x480008 one-sided. Outputs are clamped to [0, 1] as the
  original's 8-bit target clamps them.
- **`_ForceNoise`**: 256 by 256 one-byte red texels (`R8Unorm`), repeating,
  linear, with a 2×2 box mip chain (rounded means) when the parameter
  word's bit 0x40 is set (nine levels, 87 381 bytes). It is made from the
  engine's noise object the first frame a force effect exists and uploaded
  again only when the object's tick reports a new picture.
- **Passes.** The original draws each effect once per supersample layer
  with that layer's depth term (`exa_game::thaum::effect_pass`); Exarust
  draws each effect once a frame and its supersample resolve does the rest.
- **Tests.** Both programs compose with Bevy's modules and validate under
  naga (`tests/surface_shader.rs`: blast, torrent, skinned with storage and
  uniform skins, rigid, strikes); both uniforms match their WGSL structs in size and
  member offsets.

## Mind sight

`exa_render::mind_sight` holds the program the Mind domain's sense overlay
and mind glow draw with; its vertex and fragment source strings are the ones
the original compiles for its sight program (`in vec3 vPos; in vec3 vNrm; in
vec2 vTex; …` and `in vec3 vrTex; in vec4 vrCol; uniform vec3 env0; …`).
`exa_game::mind::draw` draws it and supplies the original's uniforms.

- **Vertex stage** (`mind_sight.wesl`): `vrTex.x = dot(vPos, TexVecU)` and
  `vrTex.y = dot(vPos, TexVecV)`, and `vrTex.z = max(dot(vNrm, EyeVec), 0)²`
  in the sphere's own centimetre frame; Bevy's mesh stage supplies the model
  and view-projection matrices the original's `ObjMat` holds.
- **Fragment stage**: the noise picture's red channel cut by `MindCol1.w`
  and `MindCol2.w` (each `clamp((tex − threshold)·8, 0, 1)`) mixes the three
  colours, `b = clamp(1 − vrTex.z, 0, 1)` and
  `a = clamp(b·−0.3 + tex, 0, 1)` weight them, the weighted colour is
  clamped to 0–1, `FragColor.rgb = col²·4` and
  `FragColor.w = MindCol0.w·vrTex.z`. Colours, alphas and thresholds are
  `exa_game::mind::sight::entry_colours`: a living mind has
  (0.1, 0.4, 0.6), (0.8, 0.6, 0) with alpha `(1 − first)·0.6 + 0.1` and
  (0.4, 0, 0) toward (0.31, 0, 0.78) with the second's alpha; an undead mind
  has (0.3, 0.3, 0.3, 1), the same, and (0.5, 0.5, 0.6, 0.5). `MindCol0.w`
  is the overlay alpha less the entry's fade.
- **State**: the original turns the depth test off around the draw and
  applies a state block whose blend word is 0. Exarust draws the spheres in
  the transparent pass with the depth compare `Always`, no depth writes and
  both faces, blended by the fragment's alpha [guess: the blend factors and
  the cull state].
- **Geometry**: the engine's UV sphere of 16 segments and radius 15 cm with
  normals (`exa_assets::polyhedra::uv_sphere`, its triangle strips unrolled
  into a list), its points shaped once by `shape_sphere` (x × 0.8, the half
  projection on (0, −0.8, 0.2) subtracted, a `1 − z/100` taper above the
  middle and 2 cm up); its normals stay the unshaped sphere's. The object
  matrix has the head node's three axes times the record's three scales as
  its rows and the node's origin as its last row (row vectors); Bevy takes
  them as columns, in metres.
- **Picture**: the original's 256² noise object (four octaves at
  amplitude 1, both plane-building weights 0.5, then a 0.25 s period and
  target 1: `sight::ScreenNoise::field`), uploaded as one-byte `R8Unorm`
  texels (`thaum::visuals::noise_bytes`), sampled repeating and linear. It
  runs on the game clock with the effects' random generator, is made on the
  first frame an anchor exists and is uploaded only when `NoiseField::tick`
  reports a new picture.
- **Spirit screen setup**: its constructor builds a 512² one-byte luminance
  picture (262 144 bytes) and a noise object of four octaves at amplitude 1,
  both weights 0.5 and a 0.8 s period (`sight::spirit_screen_init`). The
  screen's own draw callback is not ported yet.
- **Wiring**: `MindSightAnchor` carries a body's `SightAnchor` and
  `SightEntry`; the `PostUpdate` system spawns one sphere entity and
  material an anchor and updates them in place, and removes it while the
  overlay alpha is spent. No Mind cast creates an anchor yet; without one
  the system returns before making or ticking anything.


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
- **Door portals and light reach.** Each door link's portal is a vertical
  quad on the cells' shared edge from −50 cm up to its top, spanning the
  edge's middle ± a half width: plane B's side nibble picks ±82 cm up to
  245 cm (0) or ±115 cm up to 290 cm (any other), and a special tile
  ±200 cm up to 340 cm. The original's light-reach callback takes, for a
  door link, the point ¾ of the way from its cell's centre to the cell
  behind (1.5 m past the shared edge): light passes between points in
  different rooms when the first room has an open door link into the
  second point's cell whose point lies within 2 m of the second point, or
  the same the other way round. Its light-spill callback lists the rooms
  behind the open doors of the light's own cell when the light stands
  within 2 m of that cell's centre (it computes each door's point too, but
  compares the light with the centre). Both index cells as row × width +
  column checked only against the cell count. Exarust reproduces both
  (`Rooms::light_reach`, `Rooms::light_spill`); every door still counts as
  open because placed doors are not mechanisms yet. The control-side query
  (`Rooms::door_reach`) is looser: it reads both cells without the room
  checks, so the same room, or two points both in no room, always reach;
  otherwise the first link of the first point's room into the second
  point's cell decides through its door's own flag (another flag than the
  light query's), with no distance test. A placed door binds to its link by
  its cell and the direction of its third matrix row (the quadrant of the
  engine's quick-arccosine yaw
  picks the cell behind it, with the original's widened quarter edges), and
  the body lands in both rooms' entries (`DoorBodies`); no mechanism reads
  it yet. The room manager's active-room
  flag (`RoomTracker::is_active`) is the low bit of a room's working state:
  set once the room has faded in (and while it is wanted again mid
  fade-out), clear while it fades in or out.
- **Lights in rooms.** The original gives each room's sector its own list of
  light aliases (the lights of its cells). After the rooms exist it visits
  them in order, and a room that owns at least one light copies its whole
  list (received lights included) into each room its portals lead into,
  skipping lights already there (`RoomLights`). Exarust models those
  lists, but its renderer keeps lights global, as the drawing rules ask, so
  nothing reads them yet.
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
  receive no light, fill no GI and have no collision. [missing] The original
  also draws a light's own room's lid into that light's omni shadow faces,
  so a ceiling blocks the light; Exarust's lids cast nothing, so point-light
  shadows do not include ceilings. The manager's ray queries against the active rooms' occluders or lids are
  modelled (`grid::ray_blocks`, `grid::segment_blocks`): the active rooms in
  order, a room with no shell answering no at once, and each shell through
  the engine's one-sided mesh ray test (front faces only, determinant at
  least the f32 epsilon, barycentrics checked before the division,
  0 < t < limit). Their callers, picking under the cursor and a segment
  test, are not ported.
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

- The camera node is built with a vertical field of view of 70° and, scaled
  by the original's 25 000-unit base, near 100 and far 12 500 engine units
  (1 and 125 m). A view overrides the field of view (the play zoom, the
  viewer's `--fov`). Exarust's cameras keep Bevy's planes: the play
  camera's far distance comes from the scene's visibility and fog, which
  have not been traced back to this node, so `exa_render::camera` holds the
  node's planes as data only. The 3D view is built with exposure 1 (unused
  by Exarust's shaders), bloom strength 1, post on, a six-plane clipping
  volume and no scene camera slot.
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

The creation scene also smooths the hair it shows (one Loop subdivision,
[formats/RFC.md](formats/RFC.md), Editable meshes); Exarust shows no hair
yet. `exarust model <pack> <entry> --msm` shows any model smoothed that way,
as the original's `msm` debug command smooths a scene.

## Character decomposition

A corpse's or undead's skin rots through the procedural material the
original's texture compositor adds when the decomposition amount is above
0: three passes over the skin, run as `chardecomp_d`, `chardecomp_s` and
`chardecomp_n` on the GPU there. Exarust evaluates the same arithmetic per
texel in `exa_assets::decomp` (the compositor is on the CPU), taking the
shader texels, the skin colour and the amount. The programs' GLSL is in the
shader library `textures.flb` (`exa-tool shader --shader-library
textures.flb --dump <dir>`).

* **Diffuse.** The base skin (`Tex0`) times a colour mixed from the skin
  colour toward grey by `0.4 + 0.1·amount`, times the `skindecomp` map
  (`Tex4`, fetched at the shifted coordinate) and `2 + 0.4·amount`, with
  red at 0.85 and blue pulled toward its square root (`0.75·b + 0.25·√b`).
  The green is mixed into a double-green term by `0.3·amount`, and
  `ancientdifm` (`Tex3`), weighted `0.25·amount`, tints the result. The
  seam mask's red channel (`Tex1`), square-rooted, is the weight of the
  final layer, `humandcmpf` (`Tex5`), which is exposed
  (`1 − (1 − x)/a`, `a = 1 + (w − 1)/2`), has its red and blue pulled down by
  `d·0.04` and is mixed in by the weight. A mask built from two decay
  layers is in the program but compiled out.
* **Specular.** Green only: the skin's gloss times `ancientdifm`'s green
  times 2.5, times `0.4 + (0.75 − 0.1·amount)²`, mixed toward `humandcmpg`
  by `w·(2 − w²)`, w the seam mask; the colour is `(1, g, 0)`.
* **Normal.** The decay layers' normals (`Tex1` and up, the base normal
  `Tex0`) are added as differences from the base by their weights (`lamt`,
  one per layer, the layer count the caller names). `ancientnrmm0`
  (`Tex6`)'s xy is added by `amount/(1 + amount)`, then `humandcmpn`
  (`Tex7`)'s xy is mixed in by `√w`, w the mask texel (`Tex5`). A cavity
  normal from the eight neighbouring mask texels (squared,
  sobel-weighted, normalised, z = 0.5) is added by `3·(1 − w)²`; z is
  rebuilt from the xy length. The result is stored as `0.5·n + 0.5`.

The diffuse and specular passes bind their maps from unit 2 (`humanseams`,
the head of both lists, is skipped by the binding loops) and the normal pass
binds its two maps at units 6 and 7. The seam mask `w` all three passes
read is the material's own first pass (`decomp::mask`): `humanseams`'
red times `humandcmpw`'s red at the shifted coordinate, times the amount,
smoothed twice (`a·a·(3 − 2a)`), stored in an 8-bit red target.
`decomp::created` is the amount's ordered test. The composite draws the
passes for every job whose feral level is above zero.

## Item icons

The item icon renderer (`exa_items::item_renderer`) is a private scene
and view with two renderbuffers at twice the icon size. The icon size is
a global that starts at 160 pixels. The depth renderbuffer is plain depth;
the colour renderbuffer is sRGB with alpha, or without it while the alpha
global is clear. The scene's sun points along (0.46, 0.47, 0.48) in the colour
(0.75, 0.75, 0.75, 0.5); the camera sits at (0, 0, 350) looking at the
origin, with the settings 40 and 10. The two RGBA images are twice the icon
and the icon in size; each is the payload plus a 32-byte header (16-bit
width and height, a word of 1, the format, the flags `0x20000000`, a word
of `0x1000`, the payload's offset of 32 and its size). The icon is
later framed at 70, 30 for apparel or 20 when close, with the weapon light
preset (see `exa_items::item_gui`).

**Camera and scene.** `exa_items::item_icon` turns the plan into the camera
the render uses. The icon view's constructor gives it a 40° field of view
(the character view writes 30 to the same field) and 10 cm in the field
after it [guess: the near plane]. The render starts the camera node at
(0, 0, 500) with its target at the origin and hands the framing distance F
(20, 30 or 70 cm; 0.2, 0.3 or 0.7 m here) to the engine's scene fit
(`item_renderer::frame_distance`): the larger of the box's two cross half
extents h (the second only when the first is below it, which also halves
the depth weight), bent toward F while F is above it,
`h = F·((r·0.11 + 1) − 1/(8r + 1))` with `r = h/F`, then
`distance = h / sin(fov/2) + depth half · weight` (the runtime's sine, as
the original divides by it); the camera moves to that distance from the
box's centre along +Z. The engine's fit itself puts the node it moves at
the box's centre plus the old direction from that node to the view node,
times the distance (`item_renderer::frame_node`, a tested model). A small item thus draws small in
its cell and one at least F across fills it. The plan's screen scale is
the perspective scale, `icon · 1/tan(fov/2) / distance · 0.5` [guess: the
view's scale field read as `1/tan(fov/2)`], so the plan's extents are the
camera's own. The framing presets' two numbers are the sun's azimuth and
height; the scene environment's sector builds the direction
`(cos(π(1−h))cos(−πa), sin(π(1−h)), cos(π(1−h))sin(−πa))`.

**The running renderer.** `exa_game::item_icon_render` starts at most two
pending icons a frame. It resolves the item's ground model in the object
database (a factory `@` name assembles the weapon), bakes the model's parts
with `item_icon::IconScene`, and draws them with an off-screen `Camera3d`
on the off-screen render layer (`exa_render::supersample::OFFSCREEN_LAYER`,
below the lights' shadow layers, so no scene camera ever takes it) into an
image twice the icon's side; a white 120-lux sun with the framing preset's
direction stands on the same layer. The camera and sun carry the
`Offscreen` marker, which every world system skips (scene-camera queries,
light layers, GI and its sun, cube and sun shadows, sky, fog and the scene
post-processing), and the parts cast and receive no shadow, so the shadow
linker never moves them onto the world's layer. The camera has no
post-processing [guess: the original's private view's post pass is the
screen pass below]. With no pending icon and no job the systems return at
once; the readback is asynchronous and a job is dropped after 60 frames. A GPU
readback returns the image: the alpha global's screen pass resolves each
icon pixel as a 2 × 2 linear-light average of the 2× render [guess: the
screen shader's sampler is not traced], else the Kaiser filter reduces the
image and the halo (highlighted for the variant's bit) runs. The pixels go
into the manager's atlas and the cache slot gets its cell and content half
extents. A headless App paints the same scene with the CPU rasteriser in
`item_icon` instead; a readback with no covered pixel, or one that never
arrives, also takes that CPU paint, so a missing draw still leaves an
icon. The panel draws the filled cells through the HltCol program (below).

**The render plan** (`item_renderer::render_plan`): the item is turned by
(−0.1, 0.1, 0.5) when both the apparel and weapon bits are set; its sun is
set to the weapon preset (−0.16, 0.6516) or the other (−0.21, 0.7516); a
model whose flags' low byte is 9 hides the materials named `*`; bit 31 of
the variant subdivides the meshes. The camera node is shifted (0, 500, 0)
above a target at the origin and the framing distance is 20 (close), 30
(apparel) or 70. The extents are projected from the mesh's vertices (model
flags `0x89` masked to `0x89`) or from its box's eight corners, each axis's
minimum and maximum updated in the machine's ordered compare (an unordered
value moves neither), the span taken in single precision and widened by
0.0001 in double. The projection is the engine's packed transform, which
adds the x and y terms and the z and translation terms in pairs. The span
is scaled by `icon · view scale / √(distance²) · 0.5`, given in single
precision in that order. The post process always runs; while the alpha
global is set (the default) a screen-quad pass draws the full render into
the icon and nothing else follows, otherwise the full image is filtered
down to the icon size (2× to 1×) and the halo runs, highlighted for the
variant's `0x01000000` bit. The scene's depth bounds read the camera axis
(the node matrix's second row), start both ends at the first kept node's
stored depth and give the box a radius over its whole half-extent vector.

**Resize and stream.** A UI-scale change resizes both renderbuffers and
both images to twice the new icon and the icon (the scale hook then renders
every cached icon again). An item whose object is still streaming asks the
content stream, pumping the texture loader and sleeping zero until the
object arrives; the wait gives up after `0xf0000000` retries. A streamed
request keeps its top two tag bits, writes the condition word when its
`0x20000000` bit is set, gains `0x04000000` for a weapon section, and is
inserted into the cache's first free slot with half extents of half the
rendered size plus the GUI unit.

**The panel program** (`HltCol`). The panel binds its own icon program,
whose source is `exa_items::item_gui::ITEM_ICON_SHADER` and whose uniform is
(1, 0.75, 0.45, 1). It sums a ring's alpha (three texels left, right, below
and above, and the four diagonal pairs at two), scales it by the vertex's
green and 0.75, mixes the highlight towards white by the vertex's red, adds
that over the pixel by the square of its transparency, adds the glow to the
alpha and scales the fragment by the vertex alpha. The GUI draw path
reproduces it as `SkinMode::HltCol`.

**Character thumbnails.** `exa_render::icon_render` is the character
creation renderer (`FLX_IconRender`), a different private view from the
item renderer: a 30° field of view, a 10 cm near plane, a view value of 0.8
and a cleared view flag (read as bloom and sharpen, unconfirmed), and a
target twice the icon's side. The colour renderbuffer is
`sRGB8`, or `sRGB8_ALPHA8` for a character cut-out; the destination image
is RGBA8 with flags `0x20000000`. Its private scene's ambient multiplier is
100 with the icon preset's `(0.35, 0.36, 0.37)` ambient. One instance is
reused: a new icon size resizes the image, target and view, and a walk over
the renderer's object list draws one object per step. The readback enters
the icon atlas through `IconImage::from_icon_pixels`, which flips
top-down rows (an assumption about the readback) to the atlas's bottom-up
ones. Nothing spawns this renderer yet.

## Procedural noise

The original generates its noise images on the CPU from one linear
congruential sequence, `s = s · 0x08088405 + 1` (32 bits, wrapping),
stepped before each sample (`exa_render::image_noise`). A byte sample is
the high word of `s · 255`; a float sample is `s` read as a signed integer
times 2⁻³², so it lies in −0.5..0.5.

**Byte maps** (the terrain's blend-rule noise): white bytes, then octaves
k = 1, 2, … blended in. The source of octave k is the first
(w >> k)·(h >> k) bytes of the white noise, read as an image that many
texels wide. Destination texel x takes c = trunc((x + ½)·2⁻ᵏ) and
f = x·2⁻ᵏ − c. Below ½ it blends source texels c − 1 and c by ½ − f and
½ + f, from ½ up texels c and c + 1 by 1½ − f and f − ½, wrapping; rows the
same. Single precision throughout. The new value is (old + sample)·½,
0 below 0, 255 above 255, else rounded to even. A ridge fold then maps v
below 128 to round((v/127)²·255) and the rest to the same of 255 − v, in
double precision.

**Float images** (the force and spirit powers): white floats, then
octaves k = 1, 2, … read from the first (w/2)·(h/2) samples. Destination
row y reads source rows r − 1 and r, r = (2ᵏ⁻¹ + y) >> k, weighed by
fy = (2ᵏ⁻¹ + y − r·2ᵏ + ½)/2ᵏ (wrapping above, and r past the last row
reads row 0). Each row runs in spans between neighbouring source texels:
half a span (2ᵏ⁻¹ samples) from the last texel to the first, a whole span
(2ᵏ) per pair, half a span back to the first. fx starts at (2ᵏ⁻¹ + ½)/2ᵏ
in the first span and at ½/2ᵏ in the others and steps by 2⁻ᵏ. A sample
v = d·fx + (c·(1 − fx) + (b·fx + a·(1 − fx))) is shaped by the ridge
(1 − |1.8·v|)⁴ (in doubles) and mixed in at half weight.

**The animated field** of those powers keeps four such planes and an
output. Its phase is offset + t/period (t in ms, period 1000 ms until set,
0.8 s for force). The phase fraction f weighs the three oldest planes by
amplitude · ⅔ · sin²((f + 2 − i)·π/3), which sums to the amplitude, and
the output is their sum. Meanwhile the fourth plane is built row by row
in step with f, from fresh white floats with every octave blended as
ridge · 0.27 + old · 0.45 (force sets both to 0.5). On each whole phase
the planes rotate and new white noise is drawn. The amplitude eases to
its target, +0.05 per second up and −0.1 down, and snaps within
0.11 × the tick. The tick is clamped to 0.1 s, and ticks closer than
16 ms do nothing. Exarust takes the sine from the standard library, so it
may differ from the original's by an ulp. The field is modelled; the
powers do not draw it yet.

## Skill-tree splines

The thaumaturgy skill tree draws its connections with the original's
anti-aliased line program (`exa_render::splines`). A render object holds a
pre-built index list for 2 049 quads (the table loop runs from 0 to 2 048
inclusive; two triangles each: `2i, 2i+1, 2i+3` and `2i, 2i+3, 2i+2`) and
per-vertex strip coordinates (`u` the pair index, `v` 1 on the first vertex
of a pair, 0 on the second). A spline starts at a point with its length in
segments; every added point appends two vertices on the current point,
offset either side by the normal `(−dy, dx)` of the chord from the previous
point to the new one, scaled to the line width (set by the draw state). The
curvature term is the normal dotted with the incoming segment less the
outgoing one, × −0.09 over the width squared in double precision; the
join modes zero it on the first vertex (mode 2) or the second (mode 1). Each
vertex stores four colour bytes (a new strip's colour is zero); with the
fade set, the alpha byte is the colour's alpha × (the pair's first vertex
index ÷ the segments) × ½, rounded to even and cut to its low byte. Closing
without an explicit point extends the previous point through the current one
to twice the distance, and the final pair is dropped from the draw. The draw
state's three width values are `w/2 + √(4w)`, `−(0.3w + 0.15)` in double
precision and `√(6w)`; the viewport's reciprocals are computed but never
uploaded. The fragment rule computes the soft edge from the strip
coordinate, scales the coverage by the width and multiplies the vertex
colour (bytes scaled by 0.003921568). `splines.rs` holds the CPU geometry
and the fragment rule as WGSL; no caller exists yet (the power tree's
ribbon is built and sampled on the CPU in `exa_ui::tree_draw`, not through
this render object).

## Effect shells

The bubble and force-bubble effects are unit icosahedra (poles at ±y, two
rings of five at latitude ±atan ½, the lower turned by 36°), the force
barrier the icosahedron's five-face top cap, each subdivided three times:
every edge's midpoint pushed out to unit length, every face split into its
three corner faces then its middle one (1 280 faces and 642 points for the
bubble). The subdivided cap counts 4n − 6 points, more than it makes.
`exa_assets::polyhedra` builds them. `exa_game::effects::force_dome`
turns them into the domes' meshes: both double-sided, every face reversed
then every face as built, the points scaled to the radius (the Barrier's
cap with y and z exchanged, its node then scaled 85 by 85 by 25 cm). The
force geometries' programs and uniform sets, their per-pass texture rows
(offset by the node's level of detail, 500 over the squared camera
distance) and their fades are in `effects::effect_geometry`; the five
materials' bindings, signatures and noise upload are in
`thaum::effect_material`, and their four layered passes (each layer
matrix's depth term, which the texture row above is, computed in double
for the effect material and the tendril; the layer colours, time and
brightness uniforms and the shader's texture coordinate) in
`thaum::effect_pass`, as CPU draw plans; the
original's `effectbarrier`, `effectbubble` and `effectfbrcd` programs
themselves are not ported and nothing draws the domes yet.
`effects::emissive` models the seven GI lights the effects carry
(`gi_dome`, `gi_bubble`, `gi_cuboid_fn`, `gi_cuboid`, `gi_gradline`,
`gi_multipoint`, `gi_telebubble`) with their extents, colours and world
boxes; the blast's, the torrent's, the teleporter bubble's, the Barrier's
`gi_dome`, the force Bubble's `gi_bubble` and the barricade's
`gi_cuboid_fn` are registered as `exa_render::gi::GiShape`s while their
effects live. The strikes' `gi_gradline` shape is built by
`thaum::effect_light::strikes_shape` but the strikes aura does not carry
the weapon's two line points yet; the cuboid and tendril lights wait for a
live effect.

## Rain and the portal's spray

`exa_game::effects::rain` is the original's rain: 4096 drops in a box
300 cm to each side and 600 cm tall, centred 450 cm ahead of the eye; the
drawn vertex count `round(4096·intensity)` (half as many drops); a gust
walk in double precision; heads that fall `1.5·f` cm an update
(`f = (0.4·intensity + 0.25)·ms`) and wrap at the ground with x and z from
the effect's own generator words; tails a streak behind them that
lengthens as the view tilts (`(y⁴ + 0.2)`, `y` the camera's backward-axis
height); the colour the environment's times 2.2 and
`0.075·intensity + 0.075`, its alpha `(0.5·intensity + 0.5)(|y| + 0.5)`;
and the eye's `(0.6 − 0.000667·d)⁴` fade. `exa_game::weather` runs it
live (see [Rain](#rain) below). `effects::portal_effect` spawns a portal's particles one at a
time on an 80 × 115 cm ellipse at the node's place, each starting with an
outward step of 0.0015 of its offset, with the original's 3- or 5-bit look
index draws. `effects::fire_node` builds the `_FlameMat01` node's warm
800 cm light, and the colour ramp the unit's object conversion derives
from a record byte (`flame_ramp`); the row itself is `effects::fire_row`. The row's ribbon is
built each frame from the two shifted ends of the line and the two lagging
points: 16 sections of 8 vertices, the cross-section turned toward the
interpolated lag, its width 1.5 times the line over the section count and
its far side wobbled by 0.2 of the line (multiplied in double) times one less the direction's
face-on weight, with the fixed 0.025-step texture coordinates, the
black/black/white/white vertex colours and 192 indices. The row's material
(`exa_render::flame_row`, `flame_row.wesl`) is the shipped `_FlameMat01`
fragment stage: four `EmiTex` reads over scrolled coordinates, the vertical
falloff, the `Anim` roll and fade, and a 1 × 5 `GrdTex` palette from
transparent to warm white. The draw is additive [guess: the record's blend
state is not in this path] and the original's per-supersample scissor pass
is collapsed to one draw.

### Rain

The scene weather's rain request creates the effect; every tick the rain
amount moves, the effect takes it as its intensity, and it is freed when
the rain falls to 0.01 (`exa_world::scene_env`). While the amount is above
0.01 the effect is stepped on the game clock and drawn once a frame
(`exa_game::weather`, `exa_render::rain`):
- **Placement.** The box stands 450 cm ahead of the eye, turned 2° about
  its x axis and scaled by `√(fov / 70)` (the sense of the turn is a guess).
- **Vertices.** A line list of the first `round(4096·intensity)` heads and
  tails. The colour bytes are read unnormalised: grey 160, alpha 0 at the
  head and 255 at the tail. The colour is `RainCol0.rgb × 160`; the alpha
  `RainCol0.a × (0 or 255) × (0.6 − 0.000667·d)⁴`, so along a streak it
  rises from 0 at the head to well above 1, where the target clamps it.
- **`RainCol0`.** The scene's scratch colour (the horizon under a sky, the
  fog colour in fog) × 2.2 × (0.075·I + 0.075) / 255 (0.03·I + 0.03 while
  the game is paused); alpha `(0.5·I + 0.5)(|y| + 0.5)`, y the camera's
  backward axis's height.
- **Fragment.** The frame behind the streak `t`, clamped to 1 (the
  original's 8-bit frame buffer), becomes `0.75·t + 3·max(t − 0.1, 0)²`
  plus the vertex colour: the streak darkens dim parts of the frame and
  brightens bright ones. Blended with source alpha and one minus source
  alpha, depth-tested, one window pixel wide: the original draws a
  one-pixel line in each supersample corner at window resolution, so on
  Exarust's 2× target each line is drawn twice, one target pixel apart.
- **The frame copy.** The streaks draw in Bevy's transmissive pass, which
  copies the opaque frame first; the scene cameras get
  `ScreenSpaceTransmission` while the effect exists. Cameras without it
  draw no rain. The original draws the effect once per supersample corner
  over that corner's frame; Exarust draws it once, at the supersampled
  resolution. The reflected pass (reflection level above 3) is not drawn.
- **Look.** Over a dark night ground the streaks are faint dark lines (a
  quarter darker than what lies behind them); over bright ground or sky
  they are bright lines. `EXA_RAIN_DEBUG` and `EXA_RAIN_DRAW` (see
  `docs/DEBUGGING.md`) show where they are.
- **Sound levels.** The effect's update gives the rain loop its volume,
  the drift `(0.3·gust + 0.15)·I²` (0.12 at I = 0.9 in a still gust), and
  its pitch, the fall `(0.5·gust + 0.9)·I + 0.4`; the next weather tick
  reads both (see the audio crate's README).

The mind effects build a **UV sphere** instead: `segments` columns and
`segments / 2` rows from pole to pole at an angle step 2π/segments on both
axes, (rows + 1)(segments − 1) + segments vertices, in column pairs of
triangle strips of `segments` indices. A pair's first strip runs from
column c's north pole down between columns c and c + 1 to the south pole.
Its second strip writes column c + 1's south pole, then patches its indices
to zigzag between column c + 2 (the next pair's first strip, written
later) and column c + 1, and writes column c + 1's north pole last. The
last pair's second strip instead writes column `segments` itself (the
seam, u = 1) and patches only the column c + 1 side. The normal and
position come from the engine's fast sine (its 4/π quadrant fold rounded
to nearest, then the polynomial) and the double-precision cosine, the z
terms multiplied in double; the texture coordinates tile by the two
scales (the poles half a column to the left), the tangent runs from a
column point to the next round the equator (none at the poles), and the
colour word is -1. `exa_assets::polyhedra::uv_sphere` builds it.

## Not done yet

The per-tick
tendril motion of an electric arc and its emissive texture, the looping
fire and electricity sounds, body contacts being driven from the physics
step, the jittered supersampling pattern, the sun and terrain in the GI volume,
several resident GI blocks (and their margin copies), the GI injection's
cube-shadow test and dynamic occupancy updates, vertex dirt (dirt modes
1–3), glass
refraction, effect programs (smoke, powers), the hair program, terrain
parallax and wetness, the outdoor environment cube's masked tint (the rule
is in `scene_env::tint_environment_cube`, not uploaded), the day record table
(a constant table in the executable, not decoded) and a driven time of day
and weather, and the sun light's cloud dimmer (the model has it, the light
does not apply it). A shut door leaves the room behind it hidden. A point
in the next room is shown only when the view crosses the doorway
(`rooms::seen_through`). A passage loads the level its id names. The lid
is not drawn into the lights' cube faces, and light does not yet pass
through the portals (the wall flood of [Lights](#lights) bounds it).
Software rendering here cannot afford the light shadows, so test frames use
`--no-shadows`.

## Viewport still images

`exa_ui::viewport` keeps the original `T3DViewport`'s own rules. It offers
eight supersampling levels, each a sample count and a start in the recorded
jitter table: 1, 2, 3, 4, 8, 15, 24 and 66 points, the first at the centre.
A point's jitter is `2 · scale · point / size` per axis, `scale` being the
view's projection scales. A supersampled frame renders every sample of its
level at once: sample i takes point i, the share `(i + 1) / count` (in
double precision), `0.5 − (i + 0.5) / count`, and after the first the blend
`1 − 1 / (i + 1)`. The progressive path counts its samples from one over
successive frames, but 0.9.5 sets its refresh flag on every frame, so it
never runs. A level leaving or entering zero moves the view's count of
supersampling viewports by one. A frame without jitter shifts a stereo eye
by the camera scale, right for eye 1 and left for eye 2. The camera's scale
is `(2 · fov) · 0.5` over the camera's distance to its target, held to
100–3000 (a not-a-number distance stays one).

The viewport also places the version overlay ("V 0.9.5.2g", a unit in from
the left and up from the bottom) and the frame-rate readout (its width the
main font's gap times its length, right-aligned a unit inside the client;
refreshed once more than 250 ms have passed, two decimals and " FPS"), both
drawn over a one-pixel black shadow at half alpha, and converts between the
view's pixels and its space through the projection scales. Its press picks
the cursor action whose mask the press word holds with the most bits in the
mask's low 16 (the last among equals) when no control is selected; a
running action is ended by the next press and decides its release, and in a
view drag it gets the moves until it answers 0x80. Exarust renders its
still through `exa_render::supersample`; these rules are ready for a
viewport that hosts a view inside a form.

## Still capture and vertex layouts

`exa_render::capture` is the still-image path: eight jittered samples, each
blended in at `1 − 1/(index + 1)` (the first replaces the frame) with a
projection offset of `2 · scale · jitter / size` per axis, the jitters being
the recorded table's run from entry 10 (the 8-point level's), then the post chain and a half-size downsample (`width >> 1`,
`height >> 1`) to RGBA8. The plain downsample sharpens with the original's
4-tap contrast term (`a · 48`, `1 − a²`, a quarter of the neighbour sum);
the icon variant also widens alpha from the four texels three pixels away.
The capture is not yet driven by the running renderer.

`exa_render::geometry` holds the original's planar vertex streams and CPU
index batches. A vertex buffer reserves position (12 bytes per vertex),
then, as the format word's bits ask, texture coordinates, colour (4 bytes,
not normalised), normal, tangent (a vec4), morph delta (3 floats) and a
second texture stream (2 floats), each stream holding every vertex's values
back to back. The attribute locations are 0 position, 1 texture, 2 normal,
3 colour, 4 second texture, 6 tangent and 7 morph. The texture stream is
twelve bytes per vertex only when the 0x80 and 0x20 bits are both set (else
eight with the 0x10 bit), while its attribute reads three floats whenever
0x80 and 0x10 are set; real formats set the bits together. Index batches
use 1-, 2- or 4-byte indices (encoded 1, 2, anything else) in a CPU storage
of a 12-byte header and then, per batch, a primitive mode and count (8
bytes) followed by its indices. The element-buffer storage is not modelled.
Bevy's mesh allocator and per-pipeline layouts take their place in the
running renderer, and nothing calls these rules yet.
