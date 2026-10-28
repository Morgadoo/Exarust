## ADDED Requirements

### Requirement: Effect material pass draws
Each of the four effect material passes SHALL draw nothing while the global pass count is zero or less. Otherwise it SHALL set layer 0's colour and draw the first entry point, then for each layer from 1 to the count set that layer's colour and draw the layer entry point.

#### Scenario: No pass count
- **WHEN** any of the four passes runs with a pass count of 0
- **THEN** it draws nothing

### Requirement: Effect material depth terms
Each layer's matrix depth term SHALL be `((layer + frame + offset) × step) / divisor + base − step/2`, the counters added as integers. The effect material and tendril SHALL use step 0.002 and half step 0.001 in double precision; the shader and shell the sub-object's level of detail in single precision, with offset 1.5 for the shell and 0.5 otherwise.

#### Scenario: Depth term in double
- **WHEN** the effect material pass plans layer 0 at frame 10 from a base of 0.7
- **THEN** the depth term is 0.71999997, not the single-precision 0.72

### Requirement: Effect material pass uniforms
The shader and shell passes SHALL bind a time `base + layer × step` and, while display bit 0x40 is set, a brightness `(light − layer/divisor) × scale`. Only the shader pass SHALL bind the texture coordinate `(layer + frame + 0.5)/divisor − 0.5`, and only when its location exists. The effect material and tendril passes SHALL bind neither.

#### Scenario: Divisor, not pass count
- **WHEN** the shader pass draws layer 1 of 2 at frame 0 with light 1 and scale 2
- **THEN** the brightness is 0 and the texture coordinate is 1

### Requirement: Effect material node words
The force and tendril materials SHALL set bit 0x20 in the node's flags word. The aura and shell materials SHALL set bits 0x9000 in the node's state word and replace its sub-objects with one sub-object of buffer usage 35044 whose parent is the node.

#### Scenario: Aura node
- **WHEN** a Shell cast spawns its aura node
- **THEN** its state word carries 0x9000, it has one sub-object and binds `_ForceNoise`

### Requirement: Displacement shell pass bits
The shell pass's segment bit 0x200 SHALL draw every node's parts lacking 0x80040 with colour writes off and depth writes on, thick when the node's flags word carries 0x8000. Bit 0x400 SHALL set every skin's and node's fades to one less the shown value and draw each part lacking 0x80000 with its material's fade at the shown value and a 0x100 stage switched to 0x203, then restore both.

#### Scenario: Fade restored
- **WHEN** the fade pass draws a part whose fade is 0 at a shown value of 0.25
- **THEN** the draw uses 0.25 and the part's fade is 0 afterwards

### Requirement: Long Door placement
A Long Door's move SHALL place the body's root at the destination (centimetres) converted to metres, raised by the root's height above the body's lowest part.

#### Scenario: Destination in centimetres
- **WHEN** a body with its root 0.9 m above its lowest part moves to (500, 20, −300) cm
- **THEN** its root lands at (5, 1.1, −3) m
