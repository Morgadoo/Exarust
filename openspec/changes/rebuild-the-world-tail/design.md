# Design

## Units

The original's terrain is in centimetres; Exarust stores metres, so the
terrain block tree uses a 0.2 m cell pitch and its planes are
`(mid - n/2) * spacing` with the integer half.

## Faithful quirks kept

- Leaf nodes keep their start as an integer in the second word; the port
  stores it as the float's bits.
- The root cell is the mesh box padded by 0.01; the terrain tree attaches
  with the unpadded box and translates only its split planes.
- The cost search skips an axis under 0.0001 extent, tests 15 planes at
  1/16 steps in double, and splits only when the best cost is at most the
  triangle count; above 512 triangles it takes the middle of the longest
  axis. Empty space is chopped until the cell is under 0.1.
- The clip of a triangle that vanishes leaves the first vertex as its
  bounds.
- The any-ray test projects onto the plane of the dominant normal
  component; the nearest-ray and segment tests use the original's
  determinant epsilon.
- The gravity conversion multiplies by the reciprocal of the squared tick
  count.

## Departures

- The mesh-side leaf tests are closures: the original keeps them outside
  the tree.
- A triangle's stored normal is computed as `(c - a) x (b - a)`; the
  original's mesh derives it elsewhere. The standalone root box uses the
  vertex bounds.
- Contact records of the simulated-object base are this port's own.
