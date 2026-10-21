# Spec Delta

## ADDED Requirements

### Requirement: Arc level clip
A tendril SHALL end at the hit on the first segment that meets the level, losing the branches from that segment's end point on.

#### Scenario: A wall across the third segment
- **WHEN** a tendril's points run 0, 4, 8, 12 cm along x and the level is met at 6 cm
- **THEN** the tendril keeps three points, its last at 6 cm

### Requirement: Arc character query
Each tendril segment but the last SHALL be given to the character's query whole (start, end, unit direction and length) with a value of 20 cm; the first segment that meets a limb of another character SHALL shock it, once per character when the emitter shocks once, and the tendril SHALL send a branch toward the limb from the point before.

#### Scenario: The first segment meets
- **WHEN** the first 4 cm segment meets a limb
- **THEN** the query received a length of 4 and the value 20, and the tendril branches from its first point

### Requirement: Force shell frame
A force shell SHALL open over 1.5 times its size in ms, rest, close over its size from what it shows and, the frame after it closes, leave the scene with its light; a one-shot shell SHALL close as soon as it opens and, while closing, stay fully shown. While a shell has its light the light SHALL be (0.14, 0.08, 1) times a tenth of the light curve and a closing shell's colour 2.8 times what it shows.

#### Scenario: The frame after closing
- **WHEN** a closing shell reaches zero
- **THEN** that frame still draws and the next one removes the shell

### Requirement: Rigid shell copy
A rigid shell copy SHALL move every mesh vertex out by twice the unit sum of the normals of the faces it belongs to, and SHALL keep the triangles of the visible sections through the mesh's vertex map.

#### Scenario: A vertex without faces
- **WHEN** a vertex belongs to no face
- **THEN** it stays where it is and its normal is zero

### Requirement: Shell fade plane
The shell's fade plane SHALL be the node frame's height column with an offset of the frame's height less the bottom of the node's box (the skin's for a skinned copy), all divided by the box's height and multiplied by the plane scale; a rigid box's height SHALL be its world height, lowered to its radius when greater.

#### Scenario: An upright box
- **WHEN** an upright node frame stands at height 100 over a box centred at 90 with a half height of 50 and a radius of 60
- **THEN** the plane is (0, 1/50, 0, 60/50)
