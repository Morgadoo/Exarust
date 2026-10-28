# Spec Delta

## ADDED Requirements

### Requirement: Terrain live frame hook
The system SHALL spawn a built terrain tile hidden, mark it built only once its entity exists, show it when the frame hook inserts it, insert every tile before the level shows, and after the load run the frame hook and then the block selection once a frame, skipping both while the refresh delay counts down; an unloaded tile's blocks SHALL leave the selection and hand their sets back.

#### Scenario: Every tile shows after the load
- **WHEN** a level with terrain finishes loading
- **THEN** every tile is inserted and visible

#### Scenario: An unloaded tile
- **WHEN** the frame hook unloads a tile whose blocks hold composite sets
- **THEN** the tile is hidden, its blocks become wanted and blank, and their sets return to the pools

### Requirement: Terrain light distance
The system SHALL measure a hidden, in-reach block's light distance as the largest non-negative clearance, from 0, over the light planes the centre lies beyond: the plane value `(z + (y + x)) + w` less the support, the normal taken into the box's frame by its dot products with the box's axes, the support `(z + (y + x))` of that vector times the sign-copied half extents; the sort key SHALL be the nearest light's value (65000 with none) rounded to even plus 0x10000.

#### Scenario: A sheared box
- **WHEN** a box whose x axis is (1, 1, 0) with half extents (10, 20, 30) lies 500 cm along +X from a light box of half side 100
- **THEN** its light distance is 390

### Requirement: Terrain low-set hand-out failure
The system SHALL leave the frame unchanged when a low-resolution block gets no low set, so the walk goes on to later blocks, while a failed high-set hand-out still counts as a change.

#### Scenario: No low set
- **WHEN** the first block wanting a low set gets none and a later block has a non-zero key
- **THEN** the later block is still baked this frame

### Requirement: Terrain priority slot
The system SHALL rank only the selected blocks in the priority slot, `round(6000 − d)` within 60 m, and set the ring to half the spread of the wanted, visible blocks' old priorities plus 6000; the slot SHALL NOT run every frame, so the ring otherwise advances by one on each frame that bakes.

#### Scenario: A quiet frame
- **WHEN** a frame bakes nothing after a frame that baked
- **THEN** the ring keeps its value

### Requirement: Terrain composite pass bindings
The system SHALL bind a normal, colour, gloss or displacement pass's textured lane at unit `2·lane + 1` and masked lane at `2·lane + 2`, a mask pass's masked lane and a height pass's textured lane at `lane + 1`, the blend mask at 0, and give `TexPos` the third component `2^level / 4800`.

#### Scenario: A two-layer height pass
- **WHEN** a height pass covers two layers with height maps
- **THEN** `Tex0` binds at unit 1 and `Tex1` at unit 2

### Requirement: Terrain composite set hosting
The system SHALL, when `EXA_TERRAIN_SETS=1`, create a composite set's colour, normal, gloss and half-size displacement render targets at `32 << level` the first time the live selection holds it, at most one set a frame, and drop them when the set returns to its pool; otherwise no target is created.

#### Scenario: Allocation off by default
- **WHEN** a level with terrain runs without `EXA_TERRAIN_SETS`
- **THEN** no composite set target is created
