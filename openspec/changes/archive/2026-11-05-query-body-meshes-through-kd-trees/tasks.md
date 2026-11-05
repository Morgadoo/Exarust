## 1. Trees

- [x] 1.1 Port the sphere leaf walk and the live swept walk (`KdTree::sphere_leaves`, `KdTree::sweep_live`, `SweptRecord`)
- [x] 1.2 `mesh_tree`: the count rule, the shape-builder rule, shared trees, the centimetre scale, the ray and column queries
- [x] 1.3 Pair stage: walk the mesh side's tree

## 2. Bodies

- [x] 2.1 Limb node meshes at body build, trees on the async pool
- [x] 2.2 Held items' node meshes and factory weapons' merged mesh
- [x] 2.3 Character pick through the limb trees

## 3. Sector lists

- [x] 3.1 Body grid rebuilt after the body step; static records keep their mesh
- [x] 3.2 The Height Query's mesh mode in the ground placement
- [x] 3.3 The NPC search's gathering from the lists

## 4. Checks

- [x] 4.1 Tests: tree against brute force (sphere, swept, ray, column), walk order, pair stage, pick, body grid, mesh mode, search grid
- [x] 4.2 GPU: limb pick, placement on a dead body and on a sack, NPC search, frame time
