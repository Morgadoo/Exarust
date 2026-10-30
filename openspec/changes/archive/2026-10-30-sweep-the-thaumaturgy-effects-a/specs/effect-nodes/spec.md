# Spec Delta

## ADDED Requirements

### Requirement: Flame row ribbon
A lit flame row SHALL build its ribbon each frame as 16 sections of 8 vertices between the frame-interpolated pair of ends, each section's direction the interpolated lag delta (scaled by 0.05) normalised, with 192 indices and the black, black, white, white corner colours.

#### Scenario: The ribbon's size
- **WHEN** a lit row's ribbon is built
- **THEN** it has 128 vertices and 192 indices

### Requirement: Flame row section width
A flame row section's half width SHALL be 1.5 times the line over 16 divided by the length of its direction crossed with the unit vector from the camera, and its far side SHALL be moved by twice that plus the direction's length times the line's 0.2 share (taken in double and rounded once) times one less the absolute face-on weight.

#### Scenario: A section level with the camera
- **WHEN** a row 8 cm long whose lag lies 320 cm along +x is seen from 10 m along −z at mid-frame
- **THEN** its first section spans from −0.75 to 2.3 cm along x and its faces 0.75 cm either side

### Requirement: Flame row material
The flame row SHALL draw with the shipped `_FlameMat01` fragment stage over a 1 × 5 palette running from (0, 0, 0, 0) to (255, 230, 200, 64), its `Anim` uniform the row's texture offset, twice its negative and half of one less the fade.

#### Scenario: The palette ends
- **WHEN** the palette image is built
- **THEN** it is 1 by 5 texels with the first texel clear and the last (255, 230, 200, 64)

### Requirement: Portal import and dissolve ring
A portal marker loaded into a world whose flags carry `0x10000` SHALL be replaced by a fresh marker at its place keeping its render value, its 4-byte payload read and unused and a shorter payload refused; a dissolve burst SHALL write its seeded particles into the ring's slots in order up to the capacity and clear the system's `0x2` flag.

#### Scenario: A short payload
- **WHEN** a marker carrying the import flag has a 3-byte payload
- **THEN** no marker is made

### Requirement: Portal spray life
A portal spray particle SHALL start with the particle system's full lifetime, and its look index SHALL be written only under the 3-bit or 5-bit look flags.

#### Scenario: No look flag
- **WHEN** a spray without look flags spawns into a slot whose look is 7
- **THEN** the slot's life is the system's lifetime and its look stays 7

### Requirement: Electric template tone
The electric template selection SHALL take the strength (level·0.05 + scale) + (1 − level·0.05)·curve with the level term in double and a negative level counted as zero, clamped above at one with a not-a-number kept, and the intensity roll + level·0.4 in double.

#### Scenario: A clamped strength
- **WHEN** the level is 5 and the scale 2
- **THEN** the strength is 1

### Requirement: Electric template opacity
The opacity SHALL be 0.7 − strength·0.6 in double, blended as opacity·0.3 + owner·0.7 for a positive owner alpha; for level 5 on an owner whose state word is zero it SHALL be 0.4 − curve·0.375 with the product in single precision.

#### Scenario: The peak opacity rounds its product first
- **WHEN** the peak tone's curve sample is 0.84743375
- **THEN** its opacity is 0.08221235

### Requirement: Electric brief effect
The brief effect SHALL be chosen never at level 5, always below 3, at 3 when the engine generator's draw below 2 is 0 and above 3 when its draw below 8 is 0, with lifetime 0x16 against 0x44.

#### Scenario: A level-3 draw
- **WHEN** the generator's state is 0 at level 3
- **THEN** the effect is brief

### Requirement: Force dome hum
The Bubble's frame SHALL set its hum to (0.2, 0.75) standing, (x·0.2, 1 − x·0.25) with x = 4s(2 − 4s) growing under a share of 0.25 and ((2 − |1.5 − 3.5s|)·0.4, (1 − s)·0.75) shrinking, the 0.2 and 0.4 products in double, then add its glow times the phase gain (the share growing, one less it shrinking, else 1) once to the volume and 2.25 times to the pitch. The Barrier's frame SHALL use (1 − |0.5 − 1.5s|)·0.4 shrinking with the same double products and add four and nine times its hit glow.

#### Scenario: A shrinking Bubble
- **WHEN** a Bubble at half its share with a glow of 0.5 shrinks
- **THEN** its hum is (0.7 + 0.25, 0.375 + 0.5625)

### Requirement: Force dome GI lights
While a Barrier or a force Bubble lives it SHALL carry a GI light: the Barrier's `gi_dome` in its cap node's frame (the node's place and facing, the cap's apex the shell's +z pole) with the node's 85 × 85 × 25 cm box 75 cm ahead plus 60 cm; the Bubble's `gi_bubble` at its place, radius 5 past the dome's.

#### Scenario: The cap's apex
- **WHEN** a standing Barrier at (200, 0, 0) faces +x
- **THEN** its light maps the point 100 cm ahead to the shell's +z pole at 100

### Requirement: Barricade GI light
While a barricade lives it SHALL carry a `gi_cuboid_fn` light in the wall's frame at the node's place, its extent the node's half size as its last rising or falling frame wrote it less 60 across and 40 up; the body SHALL be the shipped program: the projected texture's ridge cubed, times 6 and the box falloff, less 0.01.

#### Scenario: A risen barricade's light
- **WHEN** a barricade has fully risen with nothing in its way
- **THEN** its light's extent is (240, 60, 5)

### Requirement: Force wave mesh and charge loop
A force wave's ribbon mesh SHALL allocate eight trail slots a chunk, 896 vertices and 1344 indices; its charge loop SHALL start silent at pitch 1 with the gain (base·0.8)·roll in double.

#### Scenario: The loop's gain
- **WHEN** the loop's base is 0.5 and its draw 0.5
- **THEN** its volume is 0 and its gain 0.2

### Requirement: Effect hazards
The live force blasts and waves SHALL carry a hazard holding their place and whether they were released, kept current after each tick, that answers a viewer with the blast's or the wave's assessment.

#### Scenario: Out of sight
- **WHEN** a wave 13 m from a viewer is assessed
- **THEN** the viewer sees nothing

### Requirement: Tendril smoke program
The displacement tendril's smoke SHALL have the shipped `effectsmoke_f` body over the shared force ribbon vertex stage: three noise reads, the taper and a black output whose alpha is the smoke's, premultiplied.

#### Scenario: Smoke and ribbon differ
- **WHEN** a smoke material and a blast material are keyed
- **THEN** their keys differ and both blend premultiplied

### Requirement: Fire colour ramp
The fire unit's object conversion SHALL turn the record's ramp byte b into (20, 200) up to 0x46, ((b − 10)·2, (b + 10)·2) below 0x5A and ((2b − 100)·2, 200) from 0x5A, in wrapping byte arithmetic.

#### Scenario: A wrapping ramp
- **WHEN** the ramp byte is 0x7F
- **THEN** the pair is (52, 200)

### Requirement: Strikes light shape
The strikes' light shape SHALL be a `gi_gradline` with the strikes light's segment, gradient, point light, attenuation 10000 and colour, in the weapon node's box plus 60 cm.

#### Scenario: The point light's weight
- **WHEN** the strikes light's point carries a strength of 0.5
- **THEN** the shape's point light weight is 5
