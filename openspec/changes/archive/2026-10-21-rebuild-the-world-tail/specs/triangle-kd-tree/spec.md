## ADDED Requirements

### Requirement: Node layout
The system SHALL store a kd-tree node as two words: a leaf flag plus triangle count, or an axis in the low two bits plus the byte offset of the first child; and either the split plane or, for a leaf, an integer start into a separate leaf-index array. Children SHALL be consecutive.

#### Scenario: A leaf and a split
- **WHEN** a leaf of 3 triangles starting at 7 and a split on axis 1 with children three nodes on are built
- **THEN** the leaf reports count 3 and start 7, and the split reports axis 1 and child offset 24

### Requirement: Build
The system SHALL build a tree from a root cell of the mesh box padded by 0.01, clip each triangle to its cell before taking bounds, search 15 planes at 1/16 of the axis with the surface-area cost and split only when the best cost is at most the triangle count, split at the middle of the longest axis above 512 triangles, and make a child a leaf at 8 triangles or a 0.1 extent.

#### Scenario: Stacked triangles
- **WHEN** 16 unit triangles are stacked three metres apart in y
- **THEN** the tree has a root split on y and two leaves of 8 triangles in id order

### Requirement: Ray queries
The system SHALL answer the nearest front-face ray inside a box-clipped limit, the both-face segment with the far end included, and the boolean ray that projects onto the plane of the dominant normal component.

#### Scenario: Front and back faces
- **WHEN** a ray travels along +y at the winding's front and along -y at its back
- **THEN** the first hits at the triangle's distance and the second misses

### Requirement: Terrain block tree
The terrain block tree SHALL pack its cell range in its root word, place each plane at `(mid - n/2)` cells with the integer half, list two triangles per cell in each leaf, and move only its split planes when translated.

#### Scenario: Translation
- **WHEN** a block tree is translated by an offset
- **THEN** its split planes move on their axis and its leaves and cached bounds do not
