# Design

## Behaviour

`bounding_sphere(points) -> Option<Sphere>` runs the original's loop; `fit_bounding_sphere(points, &mut Sphere)` writes as the original does (only the radius, 4000, without points). The scan restarts at the first point whenever a step grows r², and the support set keeps every step's update, accepted or not.

Handlers by support size: one point gives the pair with q. Two points try the pair on each slot and the triangle, and keep the original's slot order for the winner (q replaces slot 1, slot 0, or joins as slot 2), taking the first candidate when none qualifies. Three and four points try pairs, triangles and tetrahedra in the original's orders; the winner's support points, in its row's order, then q, become the set.

Spheres: `sphere_of_two` (midpoint, d²/4), `circle_of_three` (circumcentre with the edge fall-backs; a flat triangle sets only r²), `sphere_of_four` (adjugate of the edge Gram matrix; a flat tetrahedron gives centre 0). Every sum is written in the original's order and with no fused multiply-add, so f64 results match the SSE2 code bit for bit.

## Model

`Sphere { centre: Vec3, radius: f32 }`, `Ball { centre: DVec3, radius_sq: f64 }`; a private support set of up to four point indices; candidate tables as stages of rows of support slots.

## Open questions

- With NaN or near-f32-max coordinates no 3- or 4-point candidate may qualify, and the original then reads past its candidate table. Exarust leaves the support set and returns a NaN r², which the scan never takes (labelled guard).
- The x87 control word the original sets around the run does not affect its SSE2 maths (assembly read); Exarust does nothing for it.

## Docs

`docs/RENDERING.md` (Sun shadow fit) holds the rules. No `docs/formats/` page changes.
