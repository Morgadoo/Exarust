## ADDED Requirements

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
