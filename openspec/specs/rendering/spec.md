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
The system SHALL turn model nodes with a fire emitter into omni lights per the original's template. A fire of size s gets colour sRGB (1, 0.68, 0.43), intensity 8√s and radius 1500√s cm (×1.25 between 0.2 and 0.35), 5 cm above the node. Its intensity flickers by ±5 % through a soft-bounded random walk.

#### Scenario: Menu torches
- **WHEN** the main menu scene is shown
- **THEN** its two torches light the walls around them in warm light

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
The system SHALL draw small fires' flames as the original's particles. Particles start at rest on a ring around the fire node and rise by buoyancy at the 83 Hz tick. They die young or once they stray past the fire's reach. Each is drawn as a diamond stretched along its motion, textured with a wisp of `FirePrt02` and coloured by the fire's keys over its life, blended premultiplied.

#### Scenario: Menu torches
- **WHEN** the main menu scene is shown at the original's camera
- **THEN** each torch shows a rising, tapering flame about as tall as the original's

### Requirement: Texture sampling
The system SHALL sample material textures as the original does at its top texture quality with 4× supersampling. Filtering is 16× anisotropic. Colour textures get a −0.75 LOD bias; normal, gloss and packed specular maps get none.

#### Scenario: Floor at a grazing angle
- **WHEN** a floor is seen at a grazing angle
- **THEN** its texture keeps fine detail instead of blurring to a low mip level

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
The system SHALL cast shadows as the original does. Surfaces with opacity ≤ 0.8, or flagged by the material record, cast no shadows. Surfaces cast through the alpha-tested caster have their shadows cut; other surfaces cast whole.

#### Scenario: Cobwebs in torchlight
- **WHEN** a torch lights a wall hung with cobwebs
- **THEN** the cobwebs' shadows follow their strands instead of darkening the wall in solid sheets

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
The system SHALL keep the voxel GI block around the point 20 m in front of the camera, rebuilding it when that point moves more than two sectors, so surfaces in view take their ambient from the GI wherever the camera is in a large level.

#### Scenario: Far corner of a large level
- **WHEN** the camera looks at a room far from a large level's centre
- **THEN** the room's ambient comes from the GI volume, not the environment cube

### Requirement: Vertical light shaping
The system SHALL shape lights flagged `0x200` (under mask `0x300`) vertically by their two extra values, as the original does, on surfaces and in the GI injection. Other lights are unshaped.

#### Scenario: Unshaped light
- **WHEN** a light's flags do not ask for shaping
- **THEN** its light falls off by distance alone

### Requirement: Terrain in the GI volume
The system SHALL fill the GI occupancy below the terrain surface, as the original does, so light does not drain beneath terrain floors.

#### Scenario: Room on terrain
- **WHEN** a torch burns in a room whose floor is terrain
- **THEN** its bounce light stays in the room and reaches surfaces in shadow

### Requirement: GI light selection
The system SHALL inject into the GI the lights whose reach overlaps the current block, nearest first.

#### Scenario: Large level
- **WHEN** the camera is in a part of a level with hundreds of lights
- **THEN** the lights near it are the ones lighting the GI volume

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
The system SHALL keep static casters' cube shadow maps across frames, as the original does. A face SHALL be rendered only when it is needed (it reaches into the camera's view) and dirty (its light moved, a caster linked to it moved or changed links, or the light took a new slot). A face the renderer could not draw SHALL stay dirty. It SHALL hold distance / (R·k_c), compared against 0.995·d / (R·k_r) on receivers.

#### Scenario: Still room
- **WHEN** nothing moves in view for a frame
- **THEN** no cached cube face is rendered that frame

#### Scenario: Flickering torch
- **WHEN** a torch's intensity flickers
- **THEN** its cached faces are not rendered again

#### Scenario: Light on the first frame
- **WHEN** a light takes its slot on the first frame, before the renderer can draw cube faces
- **THEN** its faces are drawn on a later frame instead of staying empty and wholly shadowed

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
The system SHALL draw the skin of human and ancient bodies as the original's built-in skins instead of their `skinhuman` record, with the race's own textures named after the body model's stem (`<stem>difm`, `<stem>nrmm0`, `<stem>spcm`). The diffuse colour SHALL be the colour texture's stored texels times the middle skin tone (0.46, 0.47, 0.47), decoded from sRGB after the product.

#### Scenario: Lit skin in the character view
- **WHEN** `exarust character Default/Idle` runs
- **THEN** the chest facing the key light shows a mid tan near sRGB (100, 81, 67), neither clipped to white nor blooming

#### Scenario: Torchlit skin
- **WHEN** `exarust play exanima01` runs
- **THEN** the player's skin near the carried torch is warm-lit and does not blow out to white

#### Scenario: Ancient body
- **WHEN** `exarust --race 2 character` runs
- **THEN** the ancient's body shows its own desiccated skin texture, not an untextured glossy figure

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
The Kaiser table of a level SHALL use n = 8·2^(level−1) taps, h = n/2, c = 2^−level, and w = I0(4√(1−t²))/I0(4)·sinc(πcu) inside |t| < 1, zero outside, normalised to sum 1.

#### Scenario: Level one
- **WHEN** the level-1 table is built
- **THEN** it has eight taps, is symmetric, sums to 1 and has larger weights near its centre

### Requirement: Mip axis pass
The axis pass SHALL use the integer ratio src/dst, start at −((n−ratio)/2), and wrap each tap modulo the source extent or clamp it to the ends.

#### Scenario: Average, clamp and wrap
- **WHEN** two RGBA texels are averaged by half weights
- **THEN** the output is their mean
- **WHEN** four taps start before the first texel
- **THEN** the clamped pass repeats the first texel and the wrapping pass takes the last

### Requirement: Kaiser chain
The mip chain SHALL generate level L while both destination dimensions stay above zero, their sum is at least 3, and both divide the base exactly. Each level SHALL be resampled from the base with its own Kaiser table, horizontally then vertically.

#### Scenario: An eight-square and an odd size
- **WHEN** an 8×8 uniform image is chained
- **THEN** two levels are produced, 4×4 and 2×2, both uniform
- **WHEN** the image is 3×3 or 1×1
- **THEN** no level is produced

### Requirement: Kaiser texture mips
An RGBA8 texture with one stored level, power-of-two dimensions and both sides at least 4 SHALL get the Kaiser chain appended, and its mip level count SHALL become the base plus the generated levels. Every other texture SHALL keep its stored levels.

#### Scenario: One-level and stored-level textures
- **WHEN** a 4×4 RGBA8 texture stores only its base level
- **THEN** a 2×2 level is appended and the count becomes 2
- **WHEN** a texture stores two levels or has an odd size
- **THEN** no level is generated

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
The system SHALL hide a 0x21-kind node, set its state to 1 and clear its callbacks on release; the callbacks SHALL clear whether or not the node matched.

#### Scenario: Released and other
- **WHEN** a 0x21-kind node with visible 1 is released
- **THEN** visible is 0, state 1 and callbacks are gone
- **WHEN** a 0x22-kind node is released
- **THEN** its visible byte is unchanged but its callbacks are gone

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
