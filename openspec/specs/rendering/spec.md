# rendering Specification

## Purpose
Draws levels, characters and effects the way the original renderer does:
its surface shading, lights and shadows, voxel global illumination,
post-processing and supersampling, on Bevy and wgpu.

## Requirements

### Requirement: Post-processing
The system SHALL post-process every 3D view like the original. The lit scene clamps at 1. A bright pass (c + c²)·0.8 at half resolution feeds a 31-tap separable blur at a quarter of the resolution. The final pass applies the grading curve b = 0.9c² + 0.1c, c′ = 7.05b/(6.5b + 0.55), then combines the bloom as 1 − (1 − c′)(1 − (1.75·max(B − 0.03, 0))²). The output is encoded to sRGB.

#### Scenario: Mid-tones lift
- **WHEN** a pixel's lit value is 0.2
- **THEN** it is shown at 0.43 before bloom and the sRGB encoding

### Requirement: Original surface shading
The system SHALL shade models with the original's material record and maps. Colours are linearised, and legacy materials' diffuse is brightened by 1.5. Specular is GGX with Schlick Fresnel and no 1/π. Packed specular maps give intensity, gloss and metalness. Opacity maps cut materials out, emissive adds on top, and unlit materials show only their emissive.

#### Scenario: Menu scene
- **WHEN** `exarust --camera 0,5,7,0,-29.05 --fov 60 level menuscene` runs
- **THEN** the view matches the original's main menu framing, with the cobwebs drawn through their opacity maps instead of as grey sheets

### Requirement: Light units
The system SHALL light with the original's units. A point light's colour is linear(colour) × intensity × 2/π, attenuated as max(1 − 0.8·d/R, 0)⁴ and casting shadows. The ambient is the level's linear ambient colour × 16 × the environment map's level.

#### Scenario: Reach
- **WHEN** a surface is 1.25 radii or more from a light
- **THEN** that light adds nothing to it

### Requirement: Fires
The system SHALL turn model nodes with a fire emitter into fires per the original's presets. A fire of size s up to 0.35 has one emitter at the node and a light of sRGB (1, 0.68, 0.43), intensity 8√s and radius 1500√s cm (×1.25 above 0.2, compared in double precision), 5 cm above the node. Its intensity follows the emitter's smoothed live count and a soft-bounded random walk.

#### Scenario: Menu torches
- **WHEN** the main menu scene is shown
- **THEN** its two torches light the walls around them in warm light

#### Scenario: A torch's light
- **WHEN** a torch's stored size 0.2 is turned into a fire
- **THEN** its light has intensity 3.58 and radius 838.5 cm

### Requirement: Environment lighting
The system SHALL light surfaces from the environment cube `genref01r`: ambient as the cube at the normal (mip 5) × level ambient × 16, and reflections as the cube along the mirror direction at mip (rough − 0.1)·8.

#### Scenario: Dark corners
- **WHEN** a surface is out of reach of every light
- **THEN** it still shows the cube's dim ambient tinted by its diffuse colour

### Requirement: Blend words
The system SHALL blend materials as their blend word says: `0x4000` alpha-tested and drawn opaque, ONE/ONE_MINUS_SRC_ALPHA premultiplied, SRC_ALPHA/ONE additive, otherwise opaque or alpha-blended.

#### Scenario: Cobwebs
- **WHEN** a cobweb material (blend word `0x4103`) is drawn
- **THEN** it is cut out by its opacity map and drawn opaque

### Requirement: Late opaque rewrite
The system SHALL draw a blend word with destination nibble 3 opaque (source ONE, destination ZERO) when the record is in refraction mode, or when its opacity is above 0.99, its fall-off below 0.01 and its loaded flags lack the opacity layer bit, comparing in double precision.

#### Scenario: Faint glass kept blended
- **WHEN** a transparent record has opacity 0.5 and no refraction mode
- **THEN** it stays alpha-blended

#### Scenario: Opaque by opacity
- **WHEN** a transparent record has opacity 1, no fall-off and no opacity layer
- **THEN** it is drawn opaque

### Requirement: Diffuse premultiplication
The system SHALL scale a lit record's diffuse colour by its opacity when its masked mode bits are not refraction and its blend source, after the late rewrite, is ONE; an unlit record has no diffuse colour.

#### Scenario: Unlit record
- **WHEN** a record has only the emissive bit
- **THEN** its diffuse colour is zero

### Requirement: Voxel light volume
The system SHALL build a level's voxel light volume as the original does. Each mesh's GI sample points mark a block of 20 cm voxels below the level's highest point. Every frame, light propagates through the open voxels from their eight diagonal neighbours, the level's base ambient, lights and voxel lights are injected, and the result is stored as 8-bit sRGB.

#### Scenario: Block height
- **WHEN** a level is taller than 12.8 m
- **THEN** its block is 256 × 256 × 128 voxels instead of 256 × 256 × 64

### Requirement: Voxel GI shading
The system SHALL light surfaces inside the volume from it. The ambient is the volume 20 cm along the normal × the level ambient × 15 × openness. Reflections are the environment cube × that ambient × 16.

#### Scenario: Torchlit room
- **WHEN** a level with torches has been shown for its warm-up
- **THEN** walls and floor out of a torch's direct reach show its warm bounce light

#### Scenario: Outside the block
- **WHEN** a surface lies outside the GI block, or GI is turned off
- **THEN** it takes its ambient from the environment cube as before

### Requirement: Normal-map orientation
The system SHALL apply normal maps in the original's tangent frame. The tangent runs along +u and the bitangent along −v of the stored UVs. The map's red and green channels are used as stored, with z rebuilt as √(1 − x² − y²).

#### Scenario: Wall relief
- **WHEN** a torch lights a stone wall from below
- **THEN** the mortar grooves read as recessed, lit on the faces turned toward the torch

### Requirement: Flame particles
The system SHALL draw fires' flames as the original's particles. A small fire's particles start at rest on a ring around the node, rise by buoyancy at the 83 Hz tick, are pulled toward the node's axis and die young or past the fire's reach. Each is a diamond stretched along its motion, facing the line from the camera to the particles' centre, coloured by the fire's keys, blended premultiplied. A hidden flame builds no quads.

#### Scenario: Menu torches
- **WHEN** the main menu scene is shown at the original's camera
- **THEN** each torch shows a rising, tapering flame about as tall as the original's

#### Scenario: A particle at rest
- **WHEN** a torch emits its first particle at a node 50 m from the origin
- **THEN** the particle's position and previous position are equal

### Requirement: Texture sampling
The system SHALL sample material textures as the original does at the live texture quality, texture detail and super-sampling options, which at the retail user's settings (quality 5, detail 3, 4× super-sampling) give 16× anisotropic filtering, a −0.75 LOD bias for colour textures and none for normal, gloss and packed specular maps. A change of those options SHALL apply to textures decoded afterwards.

#### Scenario: Floor at a grazing angle
- **WHEN** a floor is seen at a grazing angle
- **THEN** its texture keeps fine detail instead of blurring to a low mip level

#### Scenario: Lower texture quality
- **WHEN** the live texture quality is 4 and the detail 2
- **THEN** textures decoded afterwards take the quality's mask, anisotropy level and −0.5 bias base, and detail 2

### Requirement: Parallax
The system SHALL displace the texture coordinates of materials that have a height map and a normal map, as the original does. The depth is a fixed number of height-map texels set by the height layer's amount. The shift follows the tangent-space view, in four accumulating steps.

#### Scenario: Floor relief
- **WHEN** the menu floor is seen at an angle
- **THEN** its bricks shift against their grooves with the view instead of lying flat

### Requirement: Displacement shadows
The system SHALL shadow lights by the height map on materials whose extra flags ask for it. Taps are marched toward each light in the tangent plane, and the light is lit by the share of taps the surface stands above.

#### Scenario: Grooves in torchlight
- **WHEN** a torch lights the floor at a low angle
- **THEN** the grooves between the bricks fall into shadow on the side away from the torch

### Requirement: Shadow casters
The system SHALL cast shadows as the original does. A record in refraction mode casts; otherwise a surface casts only with opacity above 0.8 (double precision) and a loaded blend destination other than ONE. A record flagged by its material (extra flag `0x4`) casts none. Surfaces cast through the alpha-tested caster have their shadows cut; other surfaces cast whole.

#### Scenario: Cobwebs in torchlight
- **WHEN** a torch lights a wall hung with cobwebs
- **THEN** the cobwebs' shadows follow their strands instead of darkening the wall in solid sheets

#### Scenario: Refracting glass
- **WHEN** a record is in refraction mode with opacity 0
- **THEN** it casts a shadow

### Requirement: Light self-exclusion
The system SHALL leave a light's own object out of that light's shadows, as the original does: the light's siblings, its parent and its grandparent cast none. A light at its object's root excludes nothing.

#### Scenario: Wall torch
- **WHEN** a torch burns in its wall holder
- **THEN** the wall shows the holder's thin bracket shadow and a narrow cone below the torch head, not a wide shadow of the whole handle

### Requirement: Point shadow filter
The system SHALL filter point-light shadows as the original does: eight taps of a fixed disc around the light direction, turned per pixel, over a radius of (softness + 1)·0.005, with a normal offset of 0.008·(1 − |N·L|)·d and distances compared 0.5 % short.

#### Scenario: Soft torch shadows
- **WHEN** a torch shadows a nearby surface
- **THEN** the shadow's edge is softened by the torch's flickering softness

### Requirement: GI around the camera
The system SHALL lay the voxel GI blocks out once per level over the level grid's built cells, in blocks of eight cells slid inside the grid (one block at cells (3, 3) without a grid), and SHALL draw the block the camera falls in by the floored offset over 48 m, or else the block nearest the point 20 m ahead, so surfaces in view take their ambient from the GI anywhere in a large level.

#### Scenario: Far corner of a large level
- **WHEN** the camera looks at a room far from a large level's centre
- **THEN** the room's ambient comes from the GI volume, not the environment cube

#### Scenario: Point late in a block
- **WHEN** the camera stands 2.625 blocks from the block grid's edge
- **THEN** the third block is drawn, not the fourth

### Requirement: Vertical light shaping
The system SHALL shape lights flagged `0x200` (under mask `0x300`) vertically by their two extra values, as the original does, on surfaces and in the GI injection. Other lights are unshaped.

#### Scenario: Unshaped light
- **WHEN** a light's flags do not ask for shaping
- **THEN** its light falls off by distance alone

### Requirement: Terrain in the GI volume
The system SHALL fill the GI occupancy below the terrain surface, as the original does, so light does not drain beneath terrain floors. With k the surface height in voxels truncated towards zero, a column with k below 0 SHALL be skipped; for k below the top slice voxel k SHALL hold round-half-even(fraction × 20) and slices 0..k−1 SHALL hold 20; otherwise k SHALL clamp to the top slice, no surface byte SHALL be written and slices 0..D−2 SHALL hold 20. The column's lowest slice SHALL be k.

#### Scenario: Room on terrain
- **WHEN** a torch burns in a room whose floor is terrain
- **THEN** its bounce light stays in the room and reaches surfaces in shadow

#### Scenario: Terrain above the lid
- **WHEN** a terrain column of a 64-slice block reaches voxel 67
- **THEN** slices 0 to 62 hold 20, slice 63 keeps the lid's 26 and the column's lowest slice is 63

#### Scenario: Terrain just under the top slice
- **WHEN** a terrain column of a 64-slice block reaches voxel 62.5
- **THEN** slice 62 holds 10, slices 0 to 61 hold 20 and slice 63 keeps 26

### Requirement: GI light selection
The system SHALL inject into each GI sector only the lights its 6 m cell records from the light flood (up to twelve), as the original does: every registered light, over its stored radius whatever its range flag, an unranged light flooding with unlimited reach. A spot-shaped light SHALL be left out of a sector it no longer carries down to. A level without a wall grid SHALL draw the scene's lights in one group of all its sectors, from the bottom slice.

#### Scenario: Large level
- **WHEN** the camera is in a part of a level with hundreds of lights
- **THEN** each sector of the GI volume is lit only by the lights its cell lists, at most twelve

#### Scenario: Unranged light
- **WHEN** a level light has no range flag
- **THEN** it reaches every cell its flood passes and its GI falls off over its stored radius

### Requirement: Post-processing sharpen
The system SHALL sharpen the frame in the final post pass as the original does with supersampling on: an unsharp mask from four diagonal half-pixel samples, weighted by their contrast, before the grading curve.

#### Scenario: Floor detail
- **WHEN** a textured floor fills the view
- **THEN** its fine detail is as crisp as in the original

### Requirement: Torch light reach
The system SHALL widen a small fire's light radius by 1.25 when its size, compared in double precision, is above 0.2 (up to 0.35), as the original does, so a torch (size 0.2 stored in 32 bits) reaches 838.5 cm.

#### Scenario: Wall torch
- **WHEN** a torch of size 0.2 burns
- **THEN** its light's radius is 838.5 cm

### Requirement: Supersampling
The system SHALL supersample the scene as the original does at its 4× setting: it is drawn at four samples per window pixel, averaged with equal weights into the window-sized image, then post-processed at window resolution.

#### Scenario: Fine texture detail
- **WHEN** a detailed floor fills the view
- **THEN** its fine detail shows cleanly, without aliasing patterns

### Requirement: No multisampling under supersampling
The system SHALL draw the supersampled scene without hardware multisampling, as the original does (`HWMultisample = 0`); the sub-frames are the only antialiasing.

#### Scenario: Supersampled frame
- **WHEN** a level is drawn with supersampling on
- **THEN** the scene image has one sample per pixel

### Requirement: Taps split over sub-frames
The system SHALL give each sub-frame of a supersampled frame two of the eight shadow-filter taps (sub-frame i takes taps 2i and 2i + 1, turned per window pixel), and SHALL split the GI lookup's taps the same way, as the original does.

#### Scenario: Supersampled pixel
- **WHEN** a window pixel is resolved from its four sub-frames
- **THEN** its shadow and GI values average all eight taps

### Requirement: Shadow-bias levels
The system SHALL give terrain and tile-grid floors shadow-bias level 3 and other grid set pieces level 1, and every other mesh the level its file stores in bits 16–18 of its render flags (level 0 when bit 19 is set). A surface of level v > 0 SHALL compare shadow distances 0.995·k_c/k_r short, with k_r = 1 − 0.005·v and k_c = k_r − 0.01.

#### Scenario: Torch light grazing a floor
- **WHEN** a torch lights a terrain floor at a grazing angle
- **THEN** the floor shows no acne from its own shadow

#### Scenario: Slab under painted ground
- **WHEN** a broken-floor slab of shipped level 2 lies 1–2 cm under the terrain and a torch lights the terrain over it
- **THEN** the terrain over the slab is lit, not shadowed by the slab

### Requirement: Shadow casters per light
The system SHALL let a mesh cast into a light's shadow map only when the two are linked as the original links them: their bounding spheres overlap, (1 − 0.8·d/R)⁴ · luma(colour · intensity) > 0.01 with d the distance from the light to the mesh's sphere, and the mesh is not in the light's own object. Links SHALL follow moving lights and meshes.

#### Scenario: Distant mesh
- **WHEN** a mesh lies within a light's range but where its weight is below 0.01
- **THEN** the mesh casts no shadow from that light

#### Scenario: Torch handle
- **WHEN** a torch's light renders its shadow map
- **THEN** the torch's own handle is left out of it, but other lights still get the handle's shadow

### Requirement: Lights per mesh
The system SHALL light each mesh that takes part in light linking only with its six strongest links, strongest by link weight, as the original lights an object. A linked mesh with no links SHALL receive no point light. Terrain tiles and meshes that cast no shadows SHALL be lit by every light in reach.

#### Scenario: Many lights in reach
- **WHEN** eight lights are linked to a mesh
- **THEN** only the six with the largest link weights light it

### Requirement: Lights stop at walls
In tile-grid levels the system SHALL flood each ranged light through the cell grid as the original does. It SHALL carry an angular window from cell to cell, narrowed by each side's opening and closed by solid walls, and SHALL record at most 12 lights per cell. It SHALL link the light to a mesh only if the mesh's angular extent overlaps the window in the mesh's cell or a neighbouring cell its box spills into.

#### Scenario: Light in the next room
- **WHEN** a light hangs in a room walled off from the camera's room
- **THEN** the meshes of the camera's room are neither lit nor shadowed by it

#### Scenario: Doorway
- **WHEN** a light shines through a doorway
- **THEN** only meshes within the beam through the door's opening are lit by it in the next room

### Requirement: Cached cube shadows
The system SHALL keep every caster's cube shadow maps across frames, characters' skinned meshes included, as the original does. A face SHALL be rendered only when it is needed (it reaches into the camera's view) and dirty, and SHALL draw only the casters whose bounding sphere touches it. A face the renderer could not draw SHALL stay dirty. It SHALL hold distance / (R·k_c), compared against 0.995·d / (R·k_r) on receivers.

#### Scenario: Still room
- **WHEN** nothing moves in view for a frame
- **THEN** no cached cube face is rendered that frame

#### Scenario: Flickering torch
- **WHEN** a torch's intensity flickers
- **THEN** its cached faces are not rendered again

#### Scenario: Light on the first frame
- **WHEN** a light takes its slot on the first frame, before the renderer can draw cube faces
- **THEN** its faces are drawn on a later frame instead of staying empty and wholly shadowed

### Requirement: Dirty cube faces
Every face of a light's cube SHALL go out of date when the light moves or takes a new slot. A caster linked, unlinked or moved SHALL put out of date the faces its bounding sphere touched and the faces it touches.

#### Scenario: Character beside a wall torch
- **WHEN** an animated character stands in one face of a wall torch's cube
- **THEN** that face is rendered again each frame with the character posed, and the torch's other faces stay cached

### Requirement: Alpha-tested casters
The system SHALL cast a surface through the alpha-tested caster when its material is alpha-tested, or when it has the opacity flag (set by the record or by a loaded opacity map) and an opacity above 0.75 whatever its blending, as the original does. The alpha-tested caster SHALL draw both sides of the surface and SHALL drop every texel whose cut texture (the opacity map, otherwise the diffuse map) has an alpha below ½, without the material's opacity.

#### Scenario: Blended cobwebs at the menu
- **WHEN** the main menu scene is shown with shadows at the original's camera, its alpha-blended cobwebs (opacity 1, with an opacity map) hanging around both wall torches
- **THEN** each torch lights its wall and the floor around it as in the original, broken only by the shadows of props such as the desk, the shelves and the skeleton

#### Scenario: Faint opacity-mapped surface
- **WHEN** a surface has an opacity layer and an opacity of 0.75 or less
- **THEN** its shadow is not cut by the map

#### Scenario: Opacity flag without an opacity layer
- **WHEN** a blended material's record sets the opacity flag but has no opacity layer, with an opacity above 0.75
- **THEN** its shadow is cut by its diffuse map's alpha and cast from both sides

### Requirement: Built-in skin
The system SHALL draw the skin of human and ancient bodies as the original's built-in skins instead of their `skinhuman` record, with the race's own textures named after the body model's stem (`<stem>difm`, `<stem>nrmm0`, `<stem>spcm`, or the women's `diff`, `nrmf0` and `spcf` for a female record). The diffuse colour SHALL be the colour texture's stored texels times the character's skin tone multiplier, decoded from sRGB after the product.

#### Scenario: Lit skin in the character view
- **WHEN** `exarust character Default/Idle` runs
- **THEN** the chest facing the key light shows a mid tan near sRGB (100, 81, 67), neither clipped to white nor blooming

#### Scenario: Torchlit skin
- **WHEN** `exarust play exanima01` runs
- **THEN** the player's skin near the carried torch is warm-lit and does not blow out to white

#### Scenario: Ancient body
- **WHEN** `exarust --race 2 character` runs
- **THEN** the ancient's body shows its own desiccated skin texture, not an untextured glossy figure

### Requirement: Skin tone
The system SHALL compute a character's skin tone multiplier as the original does from the record's two tone words: its HSV tone table decoded to linear colours, interpolated within the grid half each word falls in, divided by the base texture's colour, scaled by 0.6, encoded and scaled by 1.4. A body without a record SHALL take the middle tone.

#### Scenario: Tone words
- **WHEN** a record's tone words are both 0x8000
- **THEN** the multiplier is about (0.461, 0.469, 0.467)
- **WHEN** they are 0 and 0xFFFF
- **THEN** the multiplier is about (0.293, 0.295, 0.287), the darkest tone

### Requirement: Skin lighting
The system SHALL light built-in skin with F0 0.04 and roughness 1 − 0.8 × the gloss texture, plus a subsurface share of the diffuse colour, sRGB (1.0, 0.8, 0.46) for humans and (1.0, 0.63, 0.133) for ancients, lit by a wrapped term on the normal map at mip 2.

#### Scenario: Soft terminator
- **WHEN** a body is lit from one side in the character view
- **THEN** the skin turns warm and soft where light meets shadow instead of ending in a hard edge

### Requirement: Voxel lights
The system SHALL add every voxel light (`0x3D0C` node) of a level and of its props' and placed objects' models into the GI volume each step, warm-up included, as the original does: lin(rgb)·√intensity·3·0.1 over 6 × 6 columns around the nearest voxel corner (outside the 8-column margins) and 7 slices, falling off as max(1 − 5000·d², 0)⁴, reaching about 1.2 voxels through geometry. Voxel lights never light a surface directly.

#### Scenario: Healing Salve on the menu shelf
- **WHEN** the main menu scene is shown at the original's camera after the GI warm-up
- **THEN** the wall, shelf and cobwebs around the Healing Salve on the left shelf are lit blue, as in the original's capture, where before only the jar glowed

#### Scenario: Reach
- **WHEN** a voxel is 3.62 voxels (72 cm) or more from a voxel light
- **THEN** that light adds nothing to it

### Requirement: Item glow by condition
The system SHALL scale an item's voxel lights directly under its model root by k = (1 − c/255)², c the condition byte (bits 16–23) of its placement flags, and the emissive colour of the meshes directly under that root by the same k, when the item's model section sets bit 0 (salves, power crystals), as the original does. Other items and level nodes are unchanged.

#### Scenario: Spent salve
- **WHEN** `exanima02` is shown, with a salve placed with condition byte `0xEE`
- **THEN** that salve's voxel light shines at 0.0044 of its intensity, about a fifteenth of its colour, and its glow mesh at 0.0044 of its emissive colour

### Requirement: Emissive surfaces
The system SHALL draw emissive as the original does: lin(colour), doubled for materials with lit or specular bits and an emissive layer, added on top and, with an emissive map on PBR materials, weighted by N·V²·z + 1 − z (z the fall-off's amount term, else 0). Unlit materials show only their emissive, with the opacity as alpha. Additive surfaces add colour × alpha over what is behind them. The GI takes nothing from emissive surfaces.

#### Scenario: Glow inside a glass jar
- **WHEN** the menu's Healing Salve is shown
- **THEN** its glass jar shows through its additive blue glow instead of being covered by an opaque blue shape

#### Scenario: Torch handle
- **WHEN** a torch is shown
- **THEN** its handle's emissive map glows at twice the record's emissive colour

### Requirement: Opacity fall-off
The system SHALL fade a surface whose record's fall-off amount exceeds 0.01 in magnitude with the view angle, as the original does: alpha += 2^((x − N·V)·y)·z, f = 2^(−1.8·exponent), a = (f + 1)·amount and y = 1.8·exponent, both negated with extra flag `0x100`, z = |a|, x = 1 when a < 0. The opacity loses (f + 1)·f; alpha is clamped to [0, 1] before blending. Unlit surfaces take it only with an emissive map.

#### Scenario: Salve glow outline
- **WHEN** the salve's glow (opacity 0, amount 1, exponent 2, flag `0x100`) is shown
- **THEN** its alpha is 1.08 where it faces the eye and 0.09 at its outline

### Requirement: Cube pipeline cache
Each mesh and key SHALL reuse its specialized pipeline id from a cache instead of specializing again. A miss SHALL specialize once and store the id; a failure SHALL not be stored.

#### Scenario: First and later draws
- **WHEN** a mesh and key are prepared for the first time
- **THEN** the specializer runs once and the id is stored
- **WHEN** the same mesh and key are prepared again
- **THEN** the stored id is reused and the specializer does not run
- **WHEN** the specializer fails
- **THEN** nothing is stored

### Requirement: Kaiser kernel
The Kaiser table of a level SHALL use n = 8·2^(level−1) taps, h = n/2, c = 2^−level, and w = I0(4√(1−t²))/I0(4)·sinc(u·π·c) where 1 − t² ≥ 0, zero elsewhere. The half width and tap position SHALL be f32, the window and sinc f64, each weight stored as f32, and the weights SHALL be divided by their f64 sum.

#### Scenario: Level one
- **WHEN** the level-1 table is built
- **THEN** it has eight taps, is symmetric, sums to 1 and has larger weights near its centre

#### Scenario: A NaN argument
- **WHEN** the Bessel series is evaluated at NaN
- **THEN** it returns NaN instead of looping

### Requirement: Mip axis pass
The axis pass SHALL use the integer ratio src/dst, start at −((n−ratio)/2), accumulate source × weight per channel in tap order, and wrap each tap modulo the source extent or clamp it to the ends.

#### Scenario: Average, clamp and wrap
- **WHEN** two RGBA texels are averaged by half weights
- **THEN** the output is their mean
- **WHEN** four taps start before the first texel
- **THEN** the clamped pass repeats the first texel and the wrapping pass takes the last

### Requirement: Kaiser chain
The mip chain SHALL generate levels from its first requested level to its last, each side shifted right by the level with a floor of one, and SHALL stop after the level whose two sides sum to two. Each level SHALL be resampled from the base with its own table, horizontally then vertically, wrapping or clamping each axis by the image's own flags (0x10000 across, 0x20000 along the height).

#### Scenario: An eight-square and an odd size
- **WHEN** an 8×8 uniform image is chained
- **THEN** three levels are produced, 4×4, 2×2 and 1×1, all uniform
- **WHEN** the image is 3×3
- **THEN** one 1×1 level is produced

#### Scenario: Clamped edges
- **WHEN** an image without wrap flags has a bright last column
- **THEN** its first level's first texel takes nothing from that column

### Requirement: Kaiser texture mips
An uncompressed texture with one stored level SHALL get the generated chain from its first uploaded level to its end when its word and the quality mask ask for mipmapping, or that level alone otherwise, and its mip level count SHALL be the levels it then holds; uploaded sides SHALL be at least one texel. A compressed texture with one stored level SHALL get no chain, also when it is decoded on the CPU. Every other texture SHALL keep its stored levels.

#### Scenario: One-level and stored-level textures
- **WHEN** a 4×4 RGBA8 texture stores only its base level
- **THEN** a 2×2 and a 1×1 level are appended and the count becomes 3
- **WHEN** a texture stores two levels, or is a single-level BC4 image decoded on the CPU
- **THEN** no level is generated

### Requirement: Mip filtering space
A generated chain SHALL be filtered in the space the stored format word picks: centred for signed data (bit 28), linear for words with bit 16 or 17, and otherwise with colour squared and taken back through the square root, alpha linear for RGBA and grey-alpha words. Block formats decoded on the CPU SHALL be filtered linearly.

#### Scenario: A checker in gamma space
- **WHEN** a 4×4 checker of 0 and 255 in a plain RGBA8 texture is chained
- **THEN** its 2×2 level's colour is about 180 and its alpha about 128

### Requirement: Arc colour and parts
The system SHALL use the spark colour (0, 53, 0), treat kind 1 below 0xfe or kind 0x11 below 0x7f as live, and kind 4 below 0xfe as an arc-spark part.

#### Scenario: Live and spark
- **WHEN** the kind word is 1 and the byte 0xfd
- **THEN** the part is live
- **WHEN** the kind word is 4 and the byte 0
- **THEN** the part is a spark part

### Requirement: Spark brightness
The system SHALL compute the random brightness as 0.8 + 0.2·roll.

#### Scenario: Two rolls
- **WHEN** the roll is 0
- **THEN** the brightness is 0.8
- **WHEN** the roll is 0.5
- **THEN** the brightness is 0.9

### Requirement: Released electrified effect
The system SHALL release an electrified item's arc by making its first arc child (kind byte 0x21) stop renewing and enter state 1, and SHALL clear the owner's hit and release callbacks whether or not a child matched.

#### Scenario: Released and other
- **WHEN** an item with a 0x22-kind child and a 0x21-kind child is released
- **THEN** the 0x21 child stops renewing and is in state 1, the other is unchanged, and the owner's callbacks are gone
- **WHEN** an item with no arc child is released
- **THEN** only the owner's callbacks clear

### Requirement: Point shadow depth range
The system SHALL render point-light shadow maps with the original's 3 cm near plane, including the maps Bevy still renders for skinned characters.

#### Scenario: Torch held close
- **WHEN** a character carries a torch whose light sits just above its hand
- **THEN** the character's shadow follows its head, torso and legs instead of smearing into a floor-wide blob

### Requirement: Light-id encoding
The system SHALL carry a light's id in its clustered depth-bias field above Bevy's default 0.08 bias, in 1e-5 steps, and SHALL decode it as round((bias − 0.08) / 1e-5).

#### Scenario: Skinned shadow bias
- **WHEN** a light with a high record id lights a skinned character
- **THEN** the field remains a valid shadow bias near 0.08 rather than the raw id

### Requirement: GI block warm-up
The system SHALL keep the block already on screen bound while a rebuilt voxel
GI block warms behind it, and SHALL swap the new block in only after its full
warm-up has run. A level change SHALL drop both blocks.

#### Scenario: Rebuild while walking
- **WHEN** the camera's GI focus moves far enough to rebuild the block
- **THEN** the floor keeps the old block's ambient while the new one warms instead of showing black patches or a circular halo

#### Scenario: First block on load
- **WHEN** a level loads and its first GI block is built
- **THEN** that block is bound immediately and the loading screen waits for its warm-up, as before

#### Scenario: New level
- **WHEN** a different level starts
- **THEN** the previous level's active and pending GI blocks are dropped before the new level's first block is bound

### Requirement: Moved lights relink nearby casters
A light that moves SHALL be relinked only against the casters whose sphere can reach it (found by the 6 m cell of their centre, the casters wider than a cell, and those already linked to it), with the same links as testing every caster.

#### Scenario: Carried torch
- **WHEN** the player walks with a torch in exanima01
- **THEN** each frame tests the torch's links against the casters near it, not every caster of the level, and the shadows are unchanged

### Requirement: Cube faces follow a moved light
When a light moves, the system SHALL work out again, from the light's new position, the cube faces each of its casters touches, so a face draws every caster it holds.

#### Scenario: Walking with a torch
- **WHEN** the player walks past crates with a carried torch
- **THEN** the crates' shadows are whole, not cut along a cube face's edge

### Requirement: Smallest enclosing sphere
The sun shadow fit's sphere SHALL be found in double precision by move-to-front, starting at the first point with radius 0. A point not in the support set and outside the sphere by more than 1e-5 (squared distance minus r²) SHALL give a new sphere from the support set and that point; a larger result SHALL replace the sphere and restart the scan. The centre and radius SHALL be narrowed to single precision.

#### Scenario: A right triangle
- **WHEN** the points are (0, 0, 0), (4, 0, 0) and (0, 3, 0)
- **THEN** the sphere is centred at (2, 1.5, 0) with radius 2.5, resting on the last two points

#### Scenario: A point within the slack
- **WHEN** the points are (0, 0, 0), (2, 0, 0) and a point 8e-6 outside their sphere in squared distance
- **THEN** the sphere stays centred at (1, 0, 0) with radius 1

### Requirement: Sphere candidates
The new sphere SHALL be chosen among the spheres through the point and one, two or three support points, in the original's fixed orders, as the smallest that holds the other support points, else the one that misses by least; the support set SHALL become the winner's points then the new point.

#### Scenario: A regular tetrahedron
- **WHEN** three corners of a regular tetrahedron are the support set and the fourth is added
- **THEN** every pair and triangle misses a corner, the circumsphere wins and the support set holds all four

### Requirement: Empty point set
Without points only the radius SHALL be written, as 4000.

#### Scenario: No points
- **WHEN** the point set is empty
- **THEN** the radius becomes 4000 and the centre is left as it was

### Requirement: Flat triangles and tetrahedra
Three points whose doubled Gram determinant is at most 1e-7 in magnitude, or four whose edge Gram determinant is, SHALL give the squared radius 3.4e38, so that they never win; a flat tetrahedron SHALL be centred at the origin. Three points SHALL fall back to an edge sphere when their circumcentre lies outside the triangle.

#### Scenario: Collinear and coplanar points
- **WHEN** three points lie on a line, or four in a plane
- **THEN** their candidate's squared radius is 3.4e38

### Requirement: Procedural noise images
The system SHALL generate noise as the original does: white noise from the sequence s = s·0x08088405 + 1 (a byte is the high word of s·255, a float s as a signed integer times 2⁻³²), byte octaves k = 1, 2, … read from the first (w >> k)·(h >> k) white bytes, blended by the original's two-half bilinear rule with (old + sample)·½ clamped and rounded to even, the ridge fold of a byte map, and ridged float octaves mixed in at half weight.

#### Scenario: White noise
- **WHEN** four bytes of white noise are drawn from seed 0
- **THEN** they are 0, 8, 219 and 51

#### Scenario: An eight-square map
- **WHEN** an 8×8 byte map is built from seed 137 with two octaves
- **THEN** its first row is 138, 143, 140, 171, 172, 131, 187, 203

#### Scenario: Ridge fold
- **WHEN** the bytes 0, 64, 127, 128 and 200 are folded
- **THEN** they become 0, 65, 255, 255 and 48

### Requirement: Animated noise field
The system SHALL cross-fade four ridged noise planes as the original's force and spirit powers do. The phase is offset + t/period. At phase fraction f the three oldest planes weigh amplitude·⅔·sin²((f + 2 − i)·π/3). The fourth plane is built row by row from fresh white noise as ridge·0.27 + old·0.45, and the planes rotate on each whole phase.

#### Scenario: Weights
- **WHEN** the field is ticked to phase fraction 0.025
- **THEN** its output is the three oldest planes weighed about 0.485, 0.515 and 0.0005

### Requirement: Noise field timing
The field's amplitude SHALL ease toward its target by 0.05 per second up and 0.1 down, or snap when within 0.11 of the tick. The tick SHALL be clamped to 0.1 s, and ticks closer than 16 ms SHALL do nothing.

#### Scenario: A short tick
- **WHEN** a field last ticked at 0 ms is ticked at 15 ms
- **THEN** nothing changes

#### Scenario: A whole phase
- **WHEN** a field with a one-second period, amplitude 1 and target 0 is ticked at 1250 ms
- **THEN** its planes rotate once, its phase fraction is 0.25 and its amplitude is 0.99

### Requirement: Caster face mask
The system SHALL give each cube-shadow caster the faces its bounding sphere touches as the original masks a link: a face is left out only when the sphere lies farther than its radius beyond one of the face's four side planes through the light, at 45° between the face's axis and its neighbours'.

#### Scenario: A sphere behind the light
- **WHEN** a sphere of radius 0.9 m lies 1 m behind a light along −X
- **THEN** it touches all six faces

#### Scenario: A point on a cube edge
- **WHEN** a sphere of radius 0 lies on the +X/+Y edge direction of a light
- **THEN** it touches the +X and +Y faces only

### Requirement: Grid floors in the GI volume
The system SHALL give each tile-grid cell's floor the original's floor heightfield points: the cell's floor meshes are merged and sampled once, in the cell's space and in centimetres, with 10 cm columns cast straight down, the highest upward face marking the 10 cm cell under it, and a point at the centre of every 20 cm cell whose four columns all mark inside it. The points SHALL be made when the level's GI inputs are collected and splatted into every block built afterwards.

#### Scenario: Flat cell
- **WHEN** a 6 m cell's floor is flat at height 0
- **THEN** it gives 30 × 30 points 10 cm under the floor, on the 20 cm lattice around the cell centre

#### Scenario: Floor made of several meshes
- **WHEN** a cell's floor is split between two meshes inside a 20 cm cell
- **THEN** the merged floor still gives that cell's point, as one mesh would

#### Scenario: Hole in the floor
- **WHEN** a cell's floor has a 2 m square hole
- **THEN** the 20 cm cells over the hole get no point

### Requirement: Generated surface points
The system SHALL generate surface points for a mesh without stored GI points as the original does: cells of a fifth of the generator's parameter around the mesh's bounds are classified as on the surface or near it by two sphere queries, near cells beside an on cell are dropped, and a greedy 5 × 5 × 5 cover (or one point per on cell) turns the rest into points, sparse cells first and the first of equal windows winning.

#### Scenario: Flat slab
- **WHEN** the generator runs at its parameter of 20 on a 20 cm box crossed by a flat surface
- **THEN** the greedy cover gives four points on the surface, a quarter in from each corner

### Requirement: Fire tiers
The system SHALL give a fire above size 0.35 a flame on a ring of round(√s·n) sources and a bed below it: 0.35–0.5 with a light 40 cm up (8/1500), 0.5–1 with one 50 cm up (12/1600), and above 1 the large pair at half the size with one 40 cm up (12/1500), whose emitters are one-shot and whose fire is removed once they finish.

#### Scenario: A medium fire
- **WHEN** a fire of size 0.8 is built
- **THEN** its flame has 4 sources on a 32 cm ring and its bed sits 16 cm below

#### Scenario: A large fire burns out
- **WHEN** a fire of size 2 has emitted its ring and its last particle has died
- **THEN** the fire and its light are removed

### Requirement: Particle emitter tick
The system SHALL tick a particle emitter at 83 Hz as the original does: the light walk and density, the ageing, the head pops, the burst refund, Verlet integration, the one-shot or finite check, the accumulator losing one two ticks in three, then spawning while it is below one, adding (83/rate)·⅔ after each round of sources.

#### Scenario: A torch's derived values
- **WHEN** the small template is scaled to 0.2
- **THEN** its rate is 50, its lifetime 103 ticks, its pool 97 and its density normaliser 25

#### Scenario: The distance cap
- **WHEN** a torch particle with 103 ticks left stands 35 cm above its node
- **THEN** its life is capped to 13

#### Scenario: An emptied ring
- **WHEN** the last two particles of a ring die in one tick
- **THEN** the tick counts three pops

### Requirement: Particle spawn
The system SHALL spawn a particle with the Verlet pair moved by the template's per-tick speed along its random direction, and its previous position lagging by 0.3 of the source's motion. Integer draws SHALL come from the engine's linear congruential sequence and unit draws from the run-time library's Mersenne twister, in the original's order.

#### Scenario: A moving source
- **WHEN** a torch's source moves 10 cm in the tick a particle spawns
- **THEN** the particle's previous position lags it by 3 cm

### Requirement: Fire colour keys
The system SHALL colour fire particles from the original's key tables as the linear pipeline loads them: the RGB of keys 1…n scaled by 0.7 through a float and back, rounded half to even, the alpha untouched, the 0.35–0.5 bed's table unscaled. A particle's colour SHALL be the keys at n·age lerped per channel and rounded half to even, odd ring slots one key later.

#### Scenario: The small fire's brightest key
- **WHEN** the small fire's key (255, 160, 94, 64) is loaded
- **THEN** it becomes (178, 112, 66, 64)

#### Scenario: The bed's peak
- **WHEN** a 0.5–1 bed particle is half way through its life on an even slot
- **THEN** its colour is (126, 70, 35, 0)

### Requirement: Carried fire
The system SHALL carry a held torch's fire with the hand as an emitter attached to the item's body: each tick its live particles move by half the hand's displacement and new particles inherit 0.3 of it.

#### Scenario: Walking with a torch
- **WHEN** the player walks carrying a lit torch
- **THEN** the flame follows the hand with a short trail instead of being left behind

### Requirement: Electric arc presets
The system SHALL build arcs from the original's presets: an electrified weapon of 20 sustained tendrils of 5 segments of 6 cm, 4 cm wide, renewed every 10–19 ticks, with a voxel light (0.6, 0.6, 1) of 16 at 180 cm; a spark burst of 26 one-shot tendrils of 5 segments of 24 cm, 6 cm wide; a level device of size d/1000 with round(l/(4·size)) segments and a point light of size/2 at 80·size cm.

#### Scenario: The weapon's ribbon
- **WHEN** a weapon arc's ribbon is built
- **THEN** each segment quad is 4 cm across, with V running across it

#### Scenario: A device
- **WHEN** a device of duration 1000 and length 200 is built
- **THEN** it has 50 segments, 10 tendrils and a point light of 0.5 at 80 cm

### Requirement: Electrified charge
The system SHALL raise an electrified item's charge by 10 a hit, or 0x80 for a part kind with bit 0x10, and the step that would reach 255 SHALL fill it and release the arc.

#### Scenario: A charge's last step
- **WHEN** a charge of 245 takes a normal hit
- **THEN** it becomes 255 and the arc is released

### Requirement: Arc record
The system SHALL read the 84-byte arc record's flags, counts, tendril lifetime in ticks, size, segment scale, tendril values, sound name (none when empty), jitter (0.1 when not positive), offset, direction and drift. A short record SHALL be an error, never a panic.

#### Scenario: A short record
- **WHEN** a 10-byte record is read
- **THEN** it is an error

### Requirement: Arc record flags
The system SHALL make a record's arc sustained, renewing every n + Random(n) ticks, when flag 1 is set, and one-shot over n ticks otherwise. Flag 0x200 SHALL add the weapon's voxel light; 0x300 SHALL register an electric flow instead of a light.

#### Scenario: A sustained record
- **WHEN** a record with flags 0x201, lifetime 25, size 1 and scale 0.5 is read
- **THEN** the arc is sustained, renewing every 25–49 ticks, with 2 cm segments, 2 cm wide and the weapon's voxel light

### Requirement: Sun shadow map size
The system SHALL give the sun one 4096² shadow map, the original's at its top shadow detail.

#### Scenario: Any sunlit level
- **WHEN** a level with a sun is shown
- **THEN** the sun's shadow map is 4096 texels on a side

### Requirement: Fitted sun map in terrain levels
In a level with a terrain the system SHALL replace the sun's cascades for each active camera with one orthographic map fitted to that camera's view; other levels SHALL keep two cascades over 60 m, and so SHALL every level when the fit is turned off.

#### Scenario: A terrain level
- **WHEN** an outdoor level with a terrain is shown
- **THEN** the sun has one shadow cascade per camera, sampled out to 125 m

#### Scenario: A level without a terrain
- **WHEN** a sunlit level without a terrain is shown
- **THEN** the sun keeps two cascades over 60 m

### Requirement: Sun fit clipping planes
The sun fit SHALL clip the camera's view volume, out to 125 m, per axis by the sign of the camera's backward axis: a negative component SHALL keep the axis at or below the high edge, a positive one at or above the low edge, and a zero or NaN one SHALL add no plane. On x and z the edges SHALL be the level box's centre plus and minus its half extent; on y they SHALL be the level's ceiling and floor when it has an environment record, and the box's x edges otherwise.

#### Scenario: Looking down and away
- **WHEN** the camera's backward axis is (0.5, 0.7, −0.5) in a level box from (−1000, −200, −500) to (3000, 400, 1500) cm with floor 0 and ceiling 340 cm
- **THEN** the volume keeps x ≥ −1000, y ≥ 0 and z ≤ 1500 cm

#### Scenario: No environment record
- **WHEN** that level has no environment record
- **THEN** the vertical limits are 3000 and −1000 cm

### Requirement: Sun fit radius steps
The fitted half extent SHALL be 200·round(r/200 + 0.4) cm, the quotient in single precision, the bias added in double precision and rounded to nearest with ties to even; it SHALL change only when that value differs and the radius is more than 60 cm above (grow) or more than 20 cm below (shrink) the current half extent, which SHALL start at 3000 cm and carry from frame to frame.

#### Scenario: A grow held back
- **WHEN** the half extent is 3000 cm and the radius is 3060 cm
- **THEN** the half extent stays 3000 cm

#### Scenario: A grow taken
- **WHEN** the half extent is 3000 cm and the radius is 3061 cm
- **THEN** the half extent becomes 3200 cm

#### Scenario: A shrink
- **WHEN** the half extent is 3000 cm and the radius is 2819 cm
- **THEN** the half extent becomes 2800 cm

### Requirement: Sun fit centre and depth
The fit SHALL rotate the sphere's centre into the light's frame (right = direction × +Y, up = right × direction, a fixed frame past |y| 0.9999), snap its x and y to multiples of 2·half extent / 4096 rounded to nearest with ties to even, and give the box ±8000 cm along the light.

#### Scenario: Snapping
- **WHEN** the half extent is 1000 cm and the centre's light-space x is 10 cm
- **THEN** x becomes 20 texels of 0.48828125 cm

### Requirement: Sun fit with too few corners
When fewer than two corners of the clipped view survive (merged within 0.1 cm²), the system SHALL keep the previous frame's fitted map.

#### Scenario: The view leaves the level
- **WHEN** the level box lies wholly behind the camera
- **THEN** the sun's map stays as the last fitted one

### Requirement: GI sector marks
The system SHALL mark, every frame, the GI cells the camera needs as the original's first pass does: a cell at most six cells from the focus cell on each axis (and from 1 to the grid size − 2), at most 36 cells squared from it and built, whose box meets the view. The box is 300 cm either side of the cell's centre and runs from its floor slice to the block's top.

#### Scenario: Cell past the reach
- **WHEN** a cell lies five cells from the focus cell on both axes
- **THEN** it is not marked, at 50 cells squared

### Requirement: GI sector box test
The system SHALL count a GI sector box out of view when its centre lies beyond a view plane by more than 848.53 cm, or by more than the box reaches towards that plane, each half extent taking the plane normal's sign on top of its own.

#### Scenario: Box past a plane
- **WHEN** a box 300 cm wide lies with its centre 500 cm beyond a view plane
- **THEN** its cell is not marked

#### Scenario: Centre past the radius
- **WHEN** a box 1280 cm high lies with its centre 900 cm beyond a plane
- **THEN** its cell is not marked

#### Scenario: Cell without geometry
- **WHEN** a cell's floor is 255, so its box's half height is negative
- **THEN** the half height reaches away from a plane whose outward normal points up

### Requirement: GI mark widening
The system SHALL draw a sector whose cell passes the first pass's window, reach and built tests and whose own first mark, or a neighbour's, is set. The neighbours are read at the cell's row-major index ± 1 and ± the grid's height.

#### Scenario: Sector in view
- **WHEN** only one cell's box meets the view
- **THEN** that cell and its four side neighbours are drawn, the diagonal ones are not

#### Scenario: Unbuilt cell
- **WHEN** a cell next to a marked one is not built
- **THEN** it is not drawn

#### Scenario: Grid deeper than wide
- **WHEN** the grid is 16 cells wide and 12 deep
- **THEN** the second pass reads the marks 12 cells before and after a cell in row-major order

### Requirement: GI undrawn voxels
The system SHALL clear the voxels of unmarked sectors, of sectors without geometry, of the margins, of the slices below a group's floor and of the top slice every step, as the original clears its target and draws only the groups, and SHALL still add the voxel lights into them.

#### Scenario: Sector leaving the view
- **WHEN** the camera turns away from a lit sector
- **THEN** the sector's light is zero from the next step, apart from voxel lights

### Requirement: GI sector groups
The system SHALL group a block's marked sectors with geometry as the original does: a sector joins the first group with the same light set (as many lights, each found, in any order) whose floor lies within two slices, lowering the group's floor, or starts a group.

#### Scenario: Same lights, near floors
- **WHEN** sectors list the same two lights in different orders with floors 46, 47 and 48
- **THEN** they form one group with floor 46

### Requirement: GI group merges
The system SHALL, in two passes, fold a later GI group into an earlier one when they share a light (or both have none), their union stays within twelve lights, and the merged fill estimate (lights + 2) × sectors × ((depth − 1) − floor) + 80 is below the two estimates apart. The later group's new lights are appended, and the group takes the lower floor.

#### Scenario: Merge that costs more
- **WHEN** two one-light groups share their light with floors 10 and 100 in a block 128 slices high
- **THEN** they stay apart, because together they cost 782 against 431 + 161

#### Scenario: Merge that costs less
- **WHEN** the floors are 10 and 14
- **THEN** they merge with floor 10, 782 against 431 + 419

### Requirement: Mip bank by format word
A generated chain SHALL filter with the uniform bank (level L: 2^L taps of 2^−L) when the format word has bit 16 set and bit 31 clear, and with the Kaiser bank otherwise.

#### Scenario: A data step
- **WHEN** a data image (ABCD8) with rows 0, 0, 255, 255 is halved
- **THEN** its level reads exactly 0 and 255
- **WHEN** the same rows are halved as a colour image
- **THEN** the step blurs into both outputs

### Requirement: Normal-map mip renormalisation
Each filtered level of a three- or four-channel image whose header flag 0x4000000 is set SHALL have its texels' first three channels normalised before conversion back to bytes, with the squares summed x, z then y, the reciprocal square root estimate, and a zero vector left zero; one- and two-channel images SHALL not be renormalised.

#### Scenario: A signed vector
- **WHEN** a flagged XYZV8 image of signed (1, 1, −1) texels is chained
- **THEN** its first level's texels are bytes 201, 201, 54

#### Scenario: A zero vector
- **WHEN** a zero vector is renormalised
- **THEN** it stays zero

### Requirement: Resample to size
The resample to an arbitrary size SHALL return level k of the image's chain when the destination shifted left by k (1 to 13) equals the source on both axes. Otherwise it SHALL filter both axes with the phase-shifted window, in the space chosen by the word and the sizes: signed words centred, data words and magnified images linear, other words squared, with RGBA8 and LUMA8 colour words squared whatever the sizes.

#### Scenario: A power-of-two ratio
- **WHEN** an 8×8 image is resampled to 2×2
- **THEN** the result equals level 2 of its chain

#### Scenario: A magnified RGB8 image
- **WHEN** a colour RGB8 image is resampled to a larger size
- **THEN** it is filtered linearly

### Requirement: Scene pass order
The renderer's model of the original's scene pass SHALL order a frame as: the clear, the alpha-tested and opaque classes and three hook lists with depth writes, then with depth writes off the decal class and the blended lists under a 2·10⁻⁵ projection depth bias, a hook list, the particles' depth-only pre-pass and their colour, the environment's weather effects and the closing hooks.

#### Scenario: Depth writes
- **WHEN** the pass order is walked
- **THEN** depth writes are on from the alpha-tested class to the third hook list and for the particle pre-pass only

### Requirement: Render list orders
The blended list SHALL be ordered by a selection that takes the lowest 4-bit layer and then the strictly highest opacity, the opacity to beat starting at −1 for each slot and reset neither by a lower layer nor by a first pick by layer; the opaque list SHALL be grouped by program key from its first entry, each slot taking the first unused entry with the current key or else the last unused entry.

#### Scenario: Grouping
- **WHEN** the opaque keys are 5, 1, 2
- **THEN** the order is 5, 2, 1

### Requirement: Super-sampled view size
The view model SHALL draw each super-sampled sub-view at the render-scale size (the window's at the default mode 0; `3·trunc(w/4)`, `2·trunc(w/3)`, `3·trunc(w/5)` or `trunc(w/2)` for modes 1–4) into a buffer twice it, up to four sub-views a pass with the original's jitters, and at the window's size without super-sampling.

#### Scenario: The 4× setting
- **WHEN** a 1600 × 900 view draws at level 3 with the default render scale
- **THEN** four sub-views of 1600 × 900 go into a 3200 × 1800 buffer

### Requirement: View projection
The view model SHALL build the frustum's half height as the near distance over cos/sin of half the field of view and the half width as that times the aspect, and SHALL project a camera point in front of the eye to `size·0.5 + scale·size·0.5·coordinate / −z` with y flipped, a point at or behind the eye to the window edge its coordinates' signs pick.

#### Scenario: 45 degrees
- **WHEN** the field of view is 45° at a near distance of 1
- **THEN** the half height is 0.4142

### Requirement: Framebuffer stacks
The view model SHALL place a viewport from the current region's origin and clip it by the region, refusing one apart from it, and SHALL place a region from the current region's origin less its scroll offset, clipped by it, up to seven regions above the root.

#### Scenario: Region overflow
- **WHEN** an eighth region is pushed
- **THEN** it is refused

### Requirement: Factory feature word
The system SHALL take a record's feature word from its loaded flags masked by its type's plug-in, replacing a word with no low nibble by the emissive bit when the opacity is above zero (and drawing nothing otherwise). Its emissive colour SHALL double only when the word keeps the emissive bit, the lit or specular bits and the emissive layer.

#### Scenario: Emissive doubling
- **WHEN** a type-0 record has the emissive, lit and emissive-layer bits
- **THEN** its emissive colour is doubled

### Requirement: Factory map binding
The system SHALL bind a diffuse map only for a lit record, an emissive map only with the emissive bit, a specular map only with `0x100004`, and a normal map only for a lit, specular, fall-off or reflection material word.

#### Scenario: Unlit diffuse layer
- **WHEN** a record has a diffuse layer but only the emissive bit
- **THEN** no diffuse map is bound

#### Scenario: Type-2 record
- **WHEN** a type-2 record has emissive, specular and normal layers
- **THEN** none of those maps is bound

### Requirement: Light flicker
A light with flag 0x10000 and without 0x20000 SHALL flicker once a frame with step = speed × frame ms × 0.01 (frame ms held to 0…16.667): its term SHALL fall by u·step at a scale of 0.9 or more, rise by u·step at −0.9 or less and move by 2·step·(u − ½) otherwise, the bounds compared in double precision; then its scale SHALL move by frame ms × 0.1 × u′ × term in double precision and be held to ±1, u and u′ being two draws of the engine's ×0x08088405 + 1 generator in that order.

#### Scenario: One frame
- **WHEN** a light with the default parameters flickers at 10 ms with the draws 1 and 0.5
- **THEN** its term is 0.05 and its scale f32(1.0 × f64(0.5 × 0.05))

### Requirement: Light flicker output
After a flicker step a light's intensity SHALL be (scale·amplitude·0.1 + 1)·base, computed in double precision, and its parent mesh's emissive scale scale × share + 1. A light without flicker or fade SHALL leave its light and mesh as loaded.

#### Scenario: The intensity
- **WHEN** a light of base intensity 2 and amplitude 0.1 reaches scale s
- **THEN** its intensity is f32((f64(s·0.1)·0.1 + 1)·2)

#### Scenario: A light at rest
- **WHEN** a light's flags hold neither 0x10000 nor 0x20000
- **THEN** its point light keeps its loaded intensity and its parent mesh's material is not copied or changed

### Requirement: Light fade
A light with flag 0x20000 SHALL draw nothing: with a positive term its scale SHALL rise by the step (in double precision) and settle at the base intensity on reaching one, otherwise fall by it and settle at amplitude × base on reaching the amplitude; settling SHALL clear 0x10000, and the parent mesh's emissive scale SHALL be scale × share.

#### Scenario: Falling to the amplitude
- **WHEN** a fading light at scale 0.12 with a negative term steps at 10 ms
- **THEN** its intensity is 0.1 × base and flag 0x10000 is cleared

### Requirement: Smoke program
Smoke and mist SHALL draw with the original's smoke program: alpha = (|N·V|·1.5 − 1 + T − 1 + vertex alpha)·0.8 over black, where T is the product of the opacity texture's alpha at (u, v + z) and at (u + x, v + z + y) for UVOfst = (x, y, z), or 0.25 without an opacity texture; UVOfst SHALL be (0.5, t + 0.5, t) with t the parent node's scale plus the scene's milliseconds mod 80 000 over 80 000, summed in double precision.

#### Scenario: Half way through the cycle
- **WHEN** the scene time is 40 000 ms under a parent of scale 1
- **THEN** UVOfst is (0.5, 2.0, 1.5)

### Requirement: Pixel-space batcher
The 2-D batcher SHALL map a pixel vertex to ((x + tx)·sx, (ty − y)·sy) for sx = 1/width, sy = 1/height, tx the current translation's x and ty the viewport height less its y, both taken when the batch begins; a translation SHALL add to the current one and save it on a 16-level stack whose top slot keeps being overwritten; every whole four vertices SHALL draw as the triangles (0, 1, 2) and (0, 2, 3), a quad's default texture coordinates being (0, 0), (1, 0), (1, 1), (0, 1).

#### Scenario: A translated batch
- **WHEN** the translation (10, 20) is set before a batch over the viewport (100, 50)–(500, 250) and the pixel (100, 50) is added
- **THEN** the vertex is (0.275, 0.65)

### Requirement: Material type table
The material factory SHALL keep five built-in types (masks 0x01FE001F, 0x09FE031F, 0x00400018, 0x09FE001F, 0x09FE001F; bases 0, 1, 2, 3, 6); its start-up SHALL reset the table to them, and registering a type on an empty table SHALL reset it first and return the new index (5 for the first). Its factory list SHALL order records by their flags' low nibble, a new record going after those of the same or a higher priority, and a material SHALL call every record whose mask meets its plug-in mask.

#### Scenario: The first plug-in
- **WHEN** a type is registered on an empty table
- **THEN** it gets index 5 and the five built-ins precede it

### Requirement: Texture-coordinate matrix
For a material record whose extra flags carry 0x40 the loader SHALL build a texture-coordinate matrix from the engine's sine s and cosine c of the negated angle: m0 = c·sx, m1 = −s·sy, m3 = s·sx, m4 = c·sy, m8 = 1, m9 = ((½ − m0·½) − m3·½) + tx, m10 = ((½ − m1·½) − m4·½) + ty; without the flag no matrix SHALL be built.

#### Scenario: Scale about the centre
- **WHEN** the angle is 0, the scale 2 and the translation 0
- **THEN** m0 and m4 are 2 and m9 and m10 are −0.5

### Requirement: Render list choice
A material's flag word SHALL choose its list in this order: bit `0x10000` the decal list, then bit `0x8000` the alpha-tested list, then the low nibble (0 opaque, 1 to 4 blended, anything else no state). The second renderer mode SHALL send every class but a blending nibble to the decal list and file a blending nibble in a list of its own.

#### Scenario: The decal bit wins
- **WHEN** a flag word has bits `0x10000` and `0x8000`
- **THEN** the decal list is chosen

### Requirement: Existing state lookup and released flags
The lookup of an existing state SHALL ignore bit `0x10000` and the renderer mode, and a released state SHALL keep the word with bits 21 and 23 cleared for detail levels below 3 and 4.

#### Scenario: The decal bit in a lookup
- **WHEN** an existing state is looked up for the word `0x10000`
- **THEN** the opaque list is found, as the low nibble says

### Requirement: Deferred draw queue
The queue SHALL hold at most 1023 entries, each in the bucket of its sector's light count, counting the enabled lights whose weight, widened to double, is not below 0.004, capped at the renderer's count and six (a NaN weight counts). Appending to a full queue SHALL flush it, and a flush SHALL remember its last entry's bucket.

#### Scenario: A full queue
- **WHEN** a draw is appended to a queue holding 1023 entries
- **THEN** the queue flushes first and then holds that draw alone

### Requirement: Draw queue order
A draw whose bucket is the one the lights were last set up for SHALL go to the head once; any other SHALL go right behind the first entry of its bucket that has a successor, or at the end, so a group reads first entry then newest first.

#### Scenario: A repeated bucket
- **WHEN** three draws of one bucket are appended to an empty queue not set up for it
- **THEN** a flush draws the first, then the third, then the second

### Requirement: Still capture plan
A still capture SHALL take eight samples, each blended in at `1 - 1/(index + 1)` (the first replaces the frame), with the projection offset `2 * scale * jitter / size` per axis from the recorded jitter table starting at entry 10, then run the post chain and downsample to half size (`width >> 1`, `height >> 1`) as RGBA8.

#### Scenario: Blend weights
- **WHEN** the eight blend weights are listed
- **THEN** they are 0, 1/2, 2/3, 3/4, 4/5, 5/6, 6/7 and 7/8

### Requirement: Planar vertex streams
A vertex buffer SHALL reserve position (12 bytes per vertex), then, as the format word's bits ask, texture coordinates, colour (4 bytes), normal, tangent (four floats), morph delta (three floats) and a second texture stream (two floats), each stream holding every vertex's values back to back. Index batches SHALL use 1-, 2- or 4-byte indices (encoded 1, 2, anything else) in a CPU storage of a 12-byte header and per batch a mode and count (8 bytes) followed by its indices.

#### Scenario: Stream offsets
- **WHEN** a format asks for position and texture coordinates for 10 vertices
- **THEN** the texture stream starts 120 bytes after the position stream

### Requirement: Post-chain fades
The screen fade SHALL run 5000 ticks with alpha `1 - (start + 5000 - now)/5000` in double precision, its desaturation the base times that alpha and its sharpen the alpha squared raised to the tenth. The desaturation fade SHALL run 3000 ticks with ramp `1 - (start + 3000 - now)/3000`; below a quarter the mask is `1 - 4 * ramp`, else `(ramp - 0.25) * 1.333`, and the pass receives the inverse; a non-zero kind adds `(1 - mask)^2 * 0.4` to the bloom.

#### Scenario: The end of the desaturation fade
- **WHEN** the ramp reaches 1
- **THEN** the mask is 0.75 * 1.333 = 0.99975 and the pass receives 0.00025

### Requirement: GI sun injection
The system SHALL inject the sun into every drawn GI sector's group ahead of its point lights, at its colour times 3 × 0.3 × 0.01 times the voxel's openness and its visibility in the sun's shadow map: the map's comparison alone under an open sky, otherwise that comparison less the one 0.01 deeper, at least 0. The sun SHALL be left out when its colour times 3 has a third component of at most 1.19e−7.

#### Scenario: Sun in a group
- **WHEN** a drawn sector lists five point lights and the sun is up
- **THEN** its group's first light is the sun and it counts towards the twelve

### Requirement: GI cube shadows
The system SHALL shadow each point light's GI injection by its cached cube shadow, comparing the distance (in block-width units) times ((bias + 0.94)/far) × 5120, formed in double precision, plus 0.01 along the direction from the light to the voxel; far is the light's radius in centimetres when its range is on, else 3.4e38. It SHALL place the light at its position over the block's width on each axis with the height over the width.

#### Scenario: Shipped detail
- **WHEN** a ranged light of radius 2 m with no quality offset is injected
- **THEN** its cube depth scale is ((0.025 + 0.94)/200) × 5120 = 24.704

#### Scenario: Light off the block's middle
- **WHEN** a light stands 6 m along world Z from a 51.2 m block's corner
- **THEN** its depth coordinate is 6 / 51.2

### Requirement: GI cube shadow bias
The system SHALL take a light's GI cube bias as the shadow detail 5 plus the light's quality offset, capped at 6, times 0.005 in double precision, or 0.025 when that sum is above 5.

#### Scenario: Lower offset
- **WHEN** a light's quality offset is −1
- **THEN** its bias is 0.02

### Requirement: GI built-cell bounds
The system SHALL bound a level grid's built cells as the original's scan does: the first built cell inside the border gives the first row and column; each later row moves the left edge to its first built cell left of it, or else checks the span between the edges, both included; either reaches the bottom edge to that row, as does a built cell right of the span, which also moves the right edge. A grid without built cells SHALL give (−4, −4) to (3, 3).

#### Scenario: Column of cells
- **WHEN** only cells (1, 1) and (1, 4) are built
- **THEN** the bounds are (1, 1) to (1, 4)

### Requirement: GI shape draw box
The system SHALL draw a registered GI shape over its centre in voxels rounded half to even and its half extents rounded up, with the columns clipped to 8 and 248, from the centre less its vertical half extent for twice it plus one slices, and SHALL draw nothing when the centre slice is below 2 or the clipped box is empty.

#### Scenario: Shape near the floor
- **WHEN** a shape's centre lies in slice 1
- **THEN** it draws nothing

### Requirement: GI dynamic occupancy
The system SHALL update a moving GI mesh's occupancy by splatting its old points with weight −20 and its new points with +20, over a region padded 1.5 voxels on each side in world voxels and rounded half to even; a thin mesh SHALL weigh a quarter and a centred one SHALL add half a voxel to the height and leave its column's lowest slice alone.

#### Scenario: Tie in the padding
- **WHEN** a point lies 4 voxels from the block's corner
- **THEN** its region runs from voxel 2 to voxel 6

### Requirement: GI slice-centre sampling
The system SHALL read the GI light and occupancy textures for voxel layer L at the slice centre (L + 0.5)/depth and SHALL measure light distances with the height (slice + 0.5)/width.

#### Scenario: Bottom layer
- **WHEN** layer 0 of a 64-slice block is propagated
- **THEN** it samples r = 0.5/64

### Requirement: GI surface sample offset
The system SHALL sample the GI volume for a surface half a voxel above its height, and for terrain and grass at the voxel's bottom, with the per-object offset's X and Z ((grid/2 − block cell) − 4) × 30/256 + 0.5 and Y (below + 0.5)/depth, or below/depth when the half voxel is dropped.

#### Scenario: Synthetic block
- **WHEN** the synthetic grid's block at cells (3, 3) is 64 slices deep with 34 below the world's zero
- **THEN** its offset is (0.1484375, 0.5390625, 0.1484375), and its Y is 0.53125 for terrain

### Requirement: Blend layer
A render state's blend layer SHALL be bits 4–7 of its blend word.

#### Scenario: High and low bits ignored
- **WHEN** the blend word is 0x0212
- **THEN** the layer is 1

### Requirement: GI block residency
The system SHALL rank the level's built GI blocks every frame by max(|f − c| − 2400, 0) per ground axis in centimetres from the focus (NaN not clamped), squared and summed, with a stable insertion sort that moves a block forward only past a strictly farther one, SHALL build next the first unbuilt or dirty block in that order, and SHALL keep built occupancies for the level.

#### Scenario: Unbuilt blocks
- **WHEN** only the first of four blocks is built and the focus is 100 m from it
- **THEN** the order is blocks 1, 2, 3, 0 and block 1 is built next

### Requirement: GI light texture handoff
The system SHALL give the light textures to the first nine blocks in creation order, and each frame SHALL give a built block among the first nine in rank order that has no texture the texture of the farthest textured block beyond them.

#### Scenario: Focus at the far end of a row
- **WHEN** twelve built blocks lie in a row 48 m apart and the focus is at the last one
- **THEN** the last three blocks take textures 0, 1 and 2 from the first three, and the nine nearest blocks step

### Requirement: GI thin meshes
The system SHALL weigh a GI mesh's points a quarter when its first surface's material has render class bit 0x4000, or has an opacity layer and an opacity above 0.75 (an ordered compare).

#### Scenario: Cut-out material
- **WHEN** a mesh's first material has an opacity layer and opacity 0.76
- **THEN** the mesh is thin, and with opacity 0.75 or NaN it is not

### Requirement: GI top slice
The system SHALL draw a sector group's light step from its floor to the block's top slice inclusive when the environment record has flag 0x20, and otherwise SHALL clear the top slice every step.

#### Scenario: Outdoor terrain arena
- **WHEN** a group with floor 3 steps in a 64-slice block of a level whose record has flag 0x20
- **THEN** it draws slices 3 to 63, and slices 3 to 62 without the flag

### Requirement: GI program bindings
The system SHALL keep the original GI manager's program table: `gi_inject_v` and `gi_inject_g` linked with `gi_inject_f`, `gi_emission` or `gi_blit_f`; the inject program binding light i's `Shadow<i>` to texture unit 2 + i, `LVox` to 0 and `OVox` to 1; the emission program binding only `OVox`, to 1; the blit program binding `LVox` to 0.

#### Scenario: Twelfth light
- **WHEN** the inject program binds the twelfth light of a group
- **THEN** it names `Shadow11` and binds it to unit 13

### Requirement: GI base ambient quality
The system SHALL scale the level record's secondary ambient by 0.001 for the GI base ambient at shading quality 4 or above, and by 0.002 below it.

#### Scenario: Low shading quality
- **WHEN** a level's secondary ambient is 40 and shading quality is 3
- **THEN** the GI base ambient is 0.08 per channel
