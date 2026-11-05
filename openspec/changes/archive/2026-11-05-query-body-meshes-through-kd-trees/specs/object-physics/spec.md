## ADDED Requirements

### Requirement: Body mesh trees
A body part's node mesh SHALL get a kd-tree when it has more than 10 and fewer than 100 000 triangles. A limb's or a weapon's fitted mesh SHALL get one at any count. The tree SHALL be built in centimetres over the part's frame and shared by every part holding the same triangle list.

#### Scenario: Counts
- **WHEN** meshes of 10, 11, 99 999 and 100 000 triangles are registered
- **THEN** only the second and third get a tree

### Requirement: Tree queries in the pair stage
The pair stage's mesh query SHALL walk the mesh side's tree:
- a sphere wholly past a plane goes to the second child;
- a sphere reaching the plane defers the second child and goes to the first;
- any other sphere goes to the first child;
- deferred children are taken last in, first out.

A swept sphere SHALL walk its leaves with its record refreshed after each hit. A mesh without a tree SHALL be tested triangle by triangle in order.

#### Scenario: The tree answers as the triangles do
- **WHEN** 400 random spheres and 400 random swept spheres are queried against a 384-triangle box mesh, through its tree and triangle by triangle
- **THEN** both find the same triangles

#### Scenario: Leaf order
- **WHEN** a sphere reaches both planes of a two-level tree
- **THEN** it visits the inner first child, the inner second child, then the root's second child

### Requirement: Sector body lists
After each body step, every body part SHALL be listed in the 600 cm world cell its mesh box's world centre falls in, in the bodies' order. Each static collider record SHALL keep the mesh it was built from.

#### Scenario: Three crates
- **WHEN** crates stand at x = 1, 7 and −0.5 m
- **THEN** they are listed in cells 0, 1 and −1, and the nine-cell scan of cell 0 gives the first, the third, then the second
