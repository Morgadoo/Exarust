## ADDED Requirements

### Requirement: Terrain blank blocks are not drawn
Every camera's draw SHALL leave out a terrain block whose composite state is blank after the frame's selection: a new block, one past the 44.4 m reach of the look-ahead point, or one that lost its set. Its view visibility SHALL be kept. This SHALL hold for the per-pixel and the composite draw alike. `EXA_TERRAIN_DRAW_BLANK=1` SHALL turn the rule off.

#### Scenario: A block past the reach
- **WHEN** a terrain block in view lies 61 m from the look-ahead point
- **THEN** no camera draws it and it casts into no shadow map

#### Scenario: Blocks in reach
- **WHEN** two blocks in view come within reach while sets are free
- **THEN** both are baked and drawn that frame

### Requirement: Terrain selection visibility bit
The composite selection SHALL read a block as visible when the live scene's visible pass drew it, for a block the live scene holds, and by its view visibility otherwise.

#### Scenario: A block behind a wall
- **WHEN** a terrain block inside the camera's frustum is in a room the visible pass did not reach
- **THEN** the selection ranks it as hidden, by its nearest light

### Requirement: Terrain selection removal
The selection's removal of a block that leaves its tile SHALL take it out of the list and hand its set back without changing its state word; the out-of-reach branch of the selection SHALL return the block to the wanted, blank state.

#### Scenario: A held block removed
- **WHEN** a block holding a high set is removed
- **THEN** the set goes back to the high pool and the block's state still reads a held high set

### Requirement: Terrain activation
The static terrain's activation SHALL run only while it is inactive: it SHALL insert the frame hook, the scene hook and per renderer the lit draw and the wireframe hook, register the composite-cache release with its 100 ms timer once in the process, resume the worker and activate its extensions in attach order.

#### Scenario: Activated twice
- **WHEN** an active terrain is activated again
- **THEN** nothing happens

## MODIFIED Requirements

### Requirement: Brush table and limit
The palette's rules and then its materials SHALL be inserted into a table sorted by their 16-byte names with A–Z lowered, by binary search (an equal name met by the search stops it there), and looked up by the same search; each rule's two children and each painted layer SHALL resolve through that lookup. A tile SHALL take at most eleven painted brushes; at the next brush found it SHALL report "Maximum brushes exceeded." and read no further layers.

#### Scenario: Underscores
- **WHEN** "b", "_a" and "a" are inserted
- **THEN** the table reads "_a", "a", "b"

#### Scenario: Twelve brushes
- **WHEN** a tile paints twelve known brushes
- **THEN** eleven are evaluated and the limit is reported

### Requirement: Terrain sun shadows from drawn blocks
A terrain block SHALL cast into the sun's shadow map only when it is not blank and the live scene's visible pass drew it this frame. A block the scene does not hold SHALL pass the second test. `EXA_TERRAIN_SHADOW_ALL=1` SHALL turn the drawn test off.

#### Scenario: Block behind the camera
- **WHEN** a terrain block inside a sun cascade is outside the main view
- **THEN** it draws nothing into the sun's map

#### Scenario: Block in view
- **WHEN** the visible pass drew a terrain block that has a composite
- **THEN** it casts into the sun's map as before

### Requirement: Terrain cube-face shadows from drawn blocks
A point light's cube face rendered in a frame SHALL leave out every blank terrain block and every one the live scene's visible pass did not draw that frame. A block the scene does not hold SHALL pass the second test. `EXA_TERRAIN_SHADOW_ALL=1` SHALL turn the drawn test off.

#### Scenario: Block out of view
- **WHEN** a dirty cube face renders while a linked terrain block is outside the main view
- **THEN** the block draws nothing into that face

#### Scenario: Block in view
- **WHEN** the visible pass drew a linked terrain block that has a composite
- **THEN** it casts into the face as before

### Requirement: Terrain extension deactivation order
The static terrain's deactivation SHALL run only while the terrain is active: it SHALL remove the hooks its activation inserted, suspend the worker, deactivate its extensions in attach order and free the composite sets.

#### Scenario: Two extensions
- **WHEN** a terrain with extensions attached as grass then a second one deactivates
- **THEN** grass deactivates first, and a second deactivation does nothing
