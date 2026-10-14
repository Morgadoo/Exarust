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
