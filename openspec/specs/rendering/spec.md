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
