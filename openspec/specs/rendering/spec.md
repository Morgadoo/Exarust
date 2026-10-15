# rendering Specification

## Purpose
TBD - created by archiving change add-render-pipeline. Update Purpose after archive.

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
The system SHALL build a level's voxel light volume as the original does. Each mesh's GI sample points mark a block of 20 cm voxels below the level's highest point. Every frame, light propagates through the open voxels from their eight diagonal neighbours, the level's base ambient and lights are injected, and the result is stored as 8-bit sRGB.

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
The system SHALL cast shadows as the original does. Alpha-tested surfaces are cut in the shadow maps the way their colour is cut. Surfaces with opacity ≤ 0.8, or flagged by the material record, cast no shadows.

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
The system SHALL give terrain and tile-grid floors shadow-bias level 3 and other grid set pieces level 1. A surface of level v > 0 SHALL compare shadow distances 0.995·k_c/k_r short, with k_r = 1 − 0.005·v and k_c = k_r − 0.01.

#### Scenario: Torch light grazing a floor
- **WHEN** a torch lights a terrain floor at a grazing angle
- **THEN** the floor shows no acne from its own shadow

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
The system SHALL keep static casters' cube shadow maps across frames, as the original does. A face SHALL be rendered only when it is needed (it reaches into the camera's view) and dirty (its light moved, a caster linked to it moved or changed links, or the light took a new slot). It SHALL hold distance / (R·k_c), compared against 0.995·d / (R·k_r) on receivers.

#### Scenario: Still room
- **WHEN** nothing moves in view for a frame
- **THEN** no cached cube face is rendered that frame

#### Scenario: Flickering torch
- **WHEN** a torch's intensity flickers
- **THEN** its cached faces are not rendered again
