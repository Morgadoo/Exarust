# scene-nodes Specification

## Purpose
Keeps the original's scene graph: nodes registered in sectors by kind, the
sectors' and the scene's bounds, each node's 6 m cell, the per-frame sector
classification through doorways and the per-mesh visibility and cut-away
fades, as rules the renderer can take its draws from.

## Requirements

### Requirement: Registration by kind
The system SHALL register a node and its subtree in a sector as the original does: every node takes the sector as its owner; a mesh joins the sector's mesh list, a light that is not a world light its light list, an auxiliary node its auxiliary list, and geometry the scene's geometry list; only meshes, those lights and auxiliary nodes are marked in the scene; lists are appended to without a duplicate check. A registration into a locked sector SHALL do nothing.

#### Scenario: A mixed subtree
- **WHEN** a plain node holding a light, a mesh, a geometry node and an auxiliary node is registered in a room
- **THEN** every node is owned by the room, the room lists the light, the mesh and the auxiliary node, the scene lists the geometry, and the plain node and the geometry are not marked in the scene
- **WHEN** the same subtree is registered again
- **THEN** the room lists the mesh twice

#### Scenario: A world light
- **WHEN** a world light is registered in a room
- **THEN** it is owned by the room and the room's light list is unchanged

### Requirement: Batch insertion
The system SHALL append each leading sector of a batch to the scene's sectors, marked in the scene, with its children registered in it. The first entry that is not a sector SHALL register the whole batch in that entry's owner, or in the scene's own sector when it has none, and end the batch.

#### Scenario: Two rooms, then two meshes
- **WHEN** a batch of two rooms (the first holding a mesh) is inserted, then a batch of two ownerless meshes
- **THEN** the scene has its own sector and the two rooms, the first room lists its mesh, and the scene's own sector lists both meshes in batch order

### Requirement: Sector bounds
The system SHALL bound a sector by its meshes as the original does: the running corners start at zero, or at the first split mesh's centre when there is one; each mesh's world-axis extents are capped at its radius; the split meshes give the first box, the other meshes carry the same corners on to the second box; box centres are `low + (high − low) · 0.5`; the union radius sums `(x² + y²) + z²` of the farther sides about the first box's world centre. A sector without meshes SHALL keep its bounds.

#### Scenario: A split mesh and another
- **WHEN** a room at the origin holds a split mesh at x = 5 m with 1 m extents and another mesh at x = 10 m with a 1 m x extent
- **THEN** the first box is centred at x = 5 m with 1 m extents, the second runs 4…11 m in x and −1…1 m in y and z, and the union radius is √38 m

#### Scenario: No split mesh
- **WHEN** a room's only mesh is at x = 10 m with a 1 m x extent
- **THEN** the first box is empty at the origin, the second is centred at 5.5 m with 5.5 m extent, and the union radius is 11 m

#### Scenario: A moved room
- **WHEN** a room at x = 100 m holds only a split mesh at x = 5 m with 1 m extents
- **THEN** the first box's world centre is at 105 m and the union radius is √(101² + 1 + 1) m

### Requirement: Scene bounds
The system SHALL write the scene's bounds into its own sector, every other sector's boxes taken as world-axis extents by its rotation: its first box starts as its own first box and grows by every other sector's first box about its world centre (extents capped at its radius); the union corners start as its own second box's centre and half extents taken as corners and grow by every other sector's second box about its unmoved centre; its second box is left unchanged.

#### Scenario: One room
- **WHEN** a scene with empty bounds has one room whose boxes are centred at x = 6 m with extents (3, 1, 1) m
- **THEN** the scene box is centred at x = 4.5 m with extents (4.5, 1, 1) m and the union radius is √(4.5² + 1 + 1) m
- **WHEN** the scene's own second box is centred at x = 1 m with a 20 m x extent
- **THEN** the union radius is √(15.5² + 1 + 1) m

### Requirement: Sector rotation
A sector's box SHALL be turned into world-axis extents by the sector's rotation, each axis summing its three absolute matrix terms times the half extents in order, for the scene bounds; the active-sector test SHALL meet the box along the same rotation.

#### Scenario: A turned room
- **WHEN** a room at x = 10 m whose boxes have half extents (3, 1, 1) m is turned a quarter about the vertical
- **THEN** the scene box runs 0…11 m in x and −3…3 m in z, and the union radius is √52.25 m

### Requirement: Node cell
The system SHALL give a node the 6 m grid cell of its box's centre as the original does: the centimetre coordinate times the single-precision 1/600, floored, plus half the grid's count, kept in one byte, with 0 raised to 1 and anything from count − 1 up lowered to count − 2.

#### Scenario: Cells and the byte
- **WHEN** a point sits at the origin of a 32 × 40 grid
- **THEN** it is in cell (16, 20)
- **WHEN** a point sits at x = −1599 m
- **THEN** its column is 5 (the byte wraps)

### Requirement: Cell spill
The system SHALL mark a node's cell side as spilling low when its box reaches more than 1 cm below the cell's low edge, otherwise high when it reaches past 601 cm, extents taken along the world axes from the box's rotation.

#### Scenario: Edges
- **WHEN** a 25 cm box sits at the origin
- **THEN** it spills low on both axes
- **WHEN** a 4 m box sits in the middle of a cell
- **THEN** only the low side spills

### Requirement: Cell handler
The system SHALL call the scene's cell handler only when a node's cell (not its spill) changes and the node hangs directly from its sector.

#### Scenario: Handler
- **WHEN** a node's box widens until it spills but stays in its cell, then moves to the next cell
- **THEN** the handler is called only for the move

### Requirement: Sector classification
The system SHALL classify each sector every frame: a sector not in the scene or hidden gets no view bit and no doorways; the scene's own sector in any other state gets every view bit and the main views; an active sector gets bit j for each main view j that meets its world box and keeps those views moved into its space.

#### Scenario: Active views
- **WHEN** an active room's box at x = 5 m is met by the second of two main views only
- **THEN** its view mask is 0b10 and it keeps that view moved by −5 m

### Requirement: Doorway frusta
The system SHALL give an adjacent sector its doorways to active sectors, clip each one whose plane faces the eye by at least 1e−6 (double precision) with the first main view into a frustum from the eye, and with any frustum set view bit 1 and a scissor bounding the projections of each face's first vertex, rounded to the nearest pixel with ties to even.

#### Scenario: A doorway seen from the active room
- **WHEN** the eye in an active room looks at a 2 m × 3 m doorway of the adjacent room whose corners project to x = ±10.5 and y = 0.5…30.5, with two main views, and a second doorway to the same room faces away
- **THEN** the adjacent room lists both doorways, has one frustum and view bit 1, and its scissor is x −10…10, y 0…30

### Requirement: Mesh visibility
The system SHALL run the hidden pass for a sector with no view bit and no doorway to an active sector, where each mesh keeps only its top mask bit, and the visible pass otherwise, where a mesh starts with the scissor bit when the sector has a scissor and is drawn and seen when one of its sector's views meets it. A moved mesh SHALL have its cell updated in grid mode.

#### Scenario: Hidden pass
- **WHEN** an active room no main view meets holds a mesh with its top mask bit and the seen bit set
- **THEN** the mesh keeps only the top bit

### Requirement: Doorway test
In the visible pass, a mesh in one of a listed doorway's two cells whose sphere comes within 20 cm of the doorway plane and whose box reaches at least 20 cm in front of it SHALL be tested against the sector beyond; when that sector sees it, it is drawn and seen without the scissor and marks its own sector seen.

#### Scenario: Straddling a doorway
- **WHEN** a 40 cm mesh of the adjacent room stands 10 cm in front of the doorway plane, in the doorway's cell
- **THEN** it is drawn and seen without the scissor bit and its room is marked seen
- **WHEN** it stands 2 m behind the plane in the doorway's other cell, inside the portal frustum
- **THEN** it is drawn and seen with the scissor bit

### Requirement: Inherited frame bits
A mesh following a fading parent SHALL keep only the seen bit; following a faded-out parent it SHALL keep only the seen bit and skip the rest of the pass, its fade included; with both it SHALL lose every bit.

#### Scenario: Parents
- **WHEN** a seen, cut-away mesh follows a fading parent for 100 ms, then a faded-out parent for 100 ms, then both
- **THEN** its mask is the seen bit with its amount at 0.3, then the seen bit with the amount unchanged, then empty

### Requirement: Cut-away fade
The system SHALL move a drawn or seen mesh's fade as the original does: not cut away, an amount above 0 drops by 0.003 per millisecond and ends at 0 at or below 0.001 (double precision), otherwise the mesh is drawn dithered; cut away, the amount rises by 0.003 per millisecond, below the target less 1.19209e−7 a drawn mesh is dithered, otherwise the amount snaps to the target and below 0.999 (double precision) a drawn mesh is drawn partially, else not drawn.

#### Scenario: Out and back
- **WHEN** a drawn mesh is cut away for 100 ms, then 300 ms
- **THEN** its amount is 0.3 and it is dithered, then the amount is 1 and it is not drawn

#### Scenario: Thresholds
- **WHEN** a fading-in mesh holds exactly 0.001 for a frame of 0 ms
- **THEN** it stays dithered at 0.001
- **WHEN** a cut-away mesh holds exactly its target less the epsilon
- **THEN** it snaps to the target

### Requirement: Subtree moves and child validation
The system SHALL move a subtree to another sector by adding the old sector's origin less the new one's to its local position, hanging it from the new sector, registering it there when it is in the scene and marking the old sector so that its lists drop the subtree at the next membership rebuild. Child validation SHALL fill an empty child slot with the last child.

#### Scenario: Between rooms
- **WHEN** a mesh at (1, 1, 1) m in a room at x = 10 m moves to a room at x = 100 m
- **THEN** its local position is (−89, 1, 1) m, its place in the world stays at x = 11 m, and after the next frame the first room no longer lists it

#### Scenario: Empty slot
- **WHEN** a node's children are A, an empty slot, B, C
- **THEN** after validation they are A, C, B

### Requirement: Directional fill
The light flood SHALL put a light that is not flooded in the first width² cells of the grid with an empty window, at most 12 lights per cell, leaving a cell that already lists the light unchanged.

#### Scenario: A tall grid
- **WHEN** a directional light fills a 3 × 5 grid
- **THEN** cells 0 to 8 list it and the last two rows do not

### Requirement: Cut-away probe
The probe SHALL test only the sectors the first view draws and only meshes whose scene word carries the cut-away candidate bit, with the segments taken into the sector's space. An overhead mesh SHALL be tested with a 10 m segment straight down onto the looked-at point, the others with the camera's segment, against the bounding sphere. A hit SHALL mark a sphere-kind or callback-kind mesh and clear any other kind; a miss SHALL clear a marked mesh.

#### Scenario: On the line and overhead
- **WHEN** candidate meshes stand on the camera's segment, 10 m beside it, and 5 m over the player, in a drawn sector offset 100 m from the world origin
- **THEN** the mesh on the segment and the mesh over the player are marked and the one beside it is not

#### Scenario: An undrawn sector
- **WHEN** the sector holding a marked mesh is not drawn by the first view
- **THEN** the probe leaves the mark as it is

### Requirement: Cut-away carry
A mark or clear SHALL carry to the mesh's parent and to its mesh children when the mesh's bits say so, and a mesh that was not marked SHALL leave them alone.

#### Scenario: Parent and child
- **WHEN** a marked mesh with both carry bits is missed, and then a child is marked by hand under the unmarked mesh
- **THEN** the parent and child are cleared, and the hand-marked child keeps its mark

### Requirement: Membership rebuild
The system SHALL drop the aliases a sector does not own as the original does: an unprocessed one is replaced by the last alias; a processed one's slot is taken by the last kept, not yet moved alias after it and its owner, if any, is marked as links changed; the unprocessed aliases follow the compacted ones. Kept processed bucket meshes SHALL have their bucket chains remapped and linked to the waiting aliases.

#### Scenario: A hole filled from the end
- **WHEN** a sector's processed aliases are A, B, C, D, B is owned by another sector, and E waits
- **THEN** the aliases are A, D, C, E, the other sector is marked, and a mesh's chain visits A, C, D, then E

### Requirement: Dropped meshes
The system SHALL drop the meshes a sector no longer owns; a dropped processed mesh SHALL lower the processed count and give its bucket faces to the kept aliases' shadow maps as needed faces.

#### Scenario: A dropped mesh
- **WHEN** a processed mesh with bucket faces 0x9 and 0x4 leaves a sector whose first alias has a shadow map needing 0x2
- **THEN** that shadow map needs 0xb and the second alias, without a shadow map, is unchanged

### Requirement: Light activation
The system SHALL activate a light whose state word is below 2: state 2, its position snapshotted unless its light word skips it (height 0 in the flat snapshot mode), its sector alias found or created (owned by the sector, moved); in grid mode its cell SHALL be stored and the flood or fill reported, otherwise an alias relink. A light out of the scene and the sector SHALL drop all its aliases.

#### Scenario: A flat snapshot without a grid
- **WHEN** a flickering light at (4, 2.5, 1) m in the flat snapshot mode activates in a scene without a grid
- **THEN** its snapshot is (4, 0, 1), an alias relink and a first flicker step are reported, and no cell flood

### Requirement: Scene list cleanup
The system SHALL free an ownerless scene on release and list one with an owner; the cleanup hook SHALL free the listed scenes not marked owned and then run the previous hook.

#### Scenario: Release and cleanup
- **WHEN** scene A and scene B are released with owners, B is marked owned, and C is never released
- **THEN** the cleanup frees A and keeps B and C
