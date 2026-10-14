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
