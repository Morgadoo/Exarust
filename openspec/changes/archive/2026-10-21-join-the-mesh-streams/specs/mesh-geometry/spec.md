## ADDED Requirements

### Requirement: Mesh part assembly
Joining a mesh part onto a destination SHALL append its vertices and triangles, join triangles whose material the destination has into that group and make a group for any other, regroup the result by material, and leave the destination's GI points and render flags as they were. A triangle-flag stream missing on a side SHALL join as 1 and an edge-bit stream as 7. A side without texture coordinates SHALL index element 0 of the joined stream.

#### Scenario: A part without texture coordinates
- **WHEN** a part without texture coordinates is joined onto a mesh whose first texture element is (0.25, 0.5)
- **THEN** the part's vertices name element 0, which is (0.25, 0.5), and the stream count stays 3

#### Scenario: Flags the destination lacks
- **WHEN** a part with flags 0x20 is joined onto a mesh without flags
- **THEN** the flags read 1 for the mesh's triangle and 0x20 for the part's

### Requirement: Shadow-pass cull selector
The mesh render word's `0x800` bit SHALL be read as the selector of front-face culling in the shadow depth pass only; the colour pass SHALL ignore it.

#### Scenario: A mesh with the bit
- **WHEN** a mesh's render word has `0x800` set
- **THEN** `shadow_cull_front` reports true and no other render state changes

### Requirement: Vertex colour counts
`exa-tool corpus colours` SHALL report, per mesh pack, the meshes with stored vertex colours, the translucent and tinted ones, the vertices with alpha below 255, and the meshes whose group materials select a dirt mode or the wound-blood mode.

#### Scenario: The character packs
- **WHEN** the command runs on the installation
- **THEN** CharExt, Characters and Components report no coloured meshes
