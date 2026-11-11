## MODIFIED Requirements

### Requirement: Smoke program
Smoke and mist SHALL draw with the original's smoke program: alpha = (|N·V|·1.5 − 1 + T − 1 + vertex alpha)·0.8 over black, where T is the product of the opacity texture's alpha at (u, v + z) and at (u + x, v + z + y) for UVOfst = (x, y, z), or 0.25 without an opacity texture; UVOfst SHALL be (0.5, t + 0.5, t) with t the parent node's scale plus the scene's milliseconds mod 80 000 over 80 000, summed in double precision.

#### Scenario: Half way through the cycle
- **WHEN** the scene time is 40 000 ms under a parent of scale 1
- **THEN** UVOfst is (0.5, 2.0, 1.5)

#### Scenario: Facing the eye
- **WHEN** a smoke surface faces the eye with an opacity term of 0.25 and a white vertex
- **THEN** its alpha is 0.6

### Requirement: Material type table
The material factory SHALL keep five built-in types (masks 0x01FE001F, 0x09FE031F, 0x00400008, 0x09FE001F, 0x09FE001F; bases 0, 1, 2, 3, 6); its start-up SHALL reset the table to them, and registering a type on an empty table SHALL reset it first and return the new index (5 for the first). Its factory list SHALL order records by their flags' low nibble, a new record going after those of the same or a higher priority, and a material SHALL call every record whose mask meets its plug-in mask.

#### Scenario: The first plug-in
- **WHEN** a type is registered on an empty table
- **THEN** it gets index 5 and the five built-ins precede it

## ADDED Requirements

### Requirement: Type-two records draw as smoke
Every material record of type 2 (the low byte of its kind word) SHALL draw with the smoke program instead of the unlit surface, as the smoke unit's start-up hook rewrites that slot of the type table before any reset copies it.

#### Scenario: The wraith's skin
- **WHEN** the wraith's body is drawn with its `wraithskin` record (type 2)
- **THEN** it is black with the smoke program's alpha, not an emissive surface

### Requirement: Smoke key words
The smoke draw's program SHALL take the render state's material word under 0x00480008 and its shader word under 0x0003C00F with the builder's quality word added as it is.

#### Scenario: The wraith at shading quality 5
- **WHEN** the state words are 0x400808 and 0x10002 and the quality word is 0x200000
- **THEN** the program's words are 0x400008 and 0x210002

### Requirement: Smoke program build rule
The smoke program SHALL be taken to build only when its shader word has UseNrm, its object word has VrtCol and the environment's fog mode is not 1; when it does not build the surface SHALL draw nothing, so a smoke surface under the level's distance fog is not drawn.

#### Scenario: An outdoor level
- **WHEN** the wraith is drawn under the level's distance fog
- **THEN** nothing of its skin or robe is drawn
