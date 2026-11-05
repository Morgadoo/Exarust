## Units

The original's trees work in centimetres. Their limits (the 0.1 leaf extent and the 0.01 box padding) are raw constants. `MeshTree` builds over the triangles scaled by 100, so the tree's shape matches, and scales every query into it. The triangles the leaves test stay in metres.

## Order

The pair stage's results now depend on the tree's leaf order, as the original's do. Hits are tested leaf by leaf, and the sphere or record is re-read after each push. A mesh's hits can come in a different order than the file order Exarust used before. Results can differ by a fraction of a millimetre when neighbouring triangles' edges are in reach.

## Stand-ins

Props keep their stand-in colliders in the physics world. The level's own Height Query already meets them. In the mesh mode, a prop's mesh gives the same height as its stand-in; ties are logged apart. A dead character has no stand-in, so the mesh mode is what finds it.

## Load

A human has 4 404 limb triangles in 18 parts. Building its trees on the main thread took about 18 ms a body, a 0.8 s stall on exanima01. The trees are now built on the async compute pool and installed in about 90 to 150 ms.
