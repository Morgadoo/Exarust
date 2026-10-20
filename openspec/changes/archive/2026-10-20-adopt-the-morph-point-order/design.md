# Design

## Behaviour

A reference mesh and a target mesh each carry a key per point (two floats, compared in the XY plane) and triangles over point ids. The identity map starts every target point on itself. The seed scans target triangles in order and, for each unvisited one, scans reference triangles for a rotation whose three keys match within `1e-8` (squared distance). A hit maps the three corners and queues the target triangle.

The flood pops a target triangle, and for each of its points walks the target's incident faces. An unvisited neighbour is rotated so the shared point starts the triangle; the reference faces incident to the mapped point are tried, likewise rotated, and the first with both remaining keys matching maps the neighbour and queues it. Points with no match keep their identity; the resulting map is applied by the existing remap.

The caller adds a part only when the target's point count equals the reference's; the part starts with zero weights, a target-minus-reference delta per point and no child positions.

## Model

`adopt_point_order(&PointOrder)` returns the permutation; `add_morph_part(reference, target)` returns `Some(MorphPartAdd)` or `None`.

## Open questions

The original pairs nodes by a `+0xd0` identity token and selects streams by a tag; both are inputs here.

## Docs

`RF_Morphing` is **76%** (13,311 of 17,099 bytes). The headline stays 14.4% and 5.7%.
