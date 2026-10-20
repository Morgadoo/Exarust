# Proposal

## Why

The original's shared intersection unit (`RF_Intersection`, 13.9 KB) holds the sphere, box, oriented-box and swept-sphere tests the physics, cloth and cameras use, the per-node cube-face mask of the point-light shadows, and the convex volumes the renderer clips portals and directional shadows with. A first pass reproduced most of it from the pseudo-code; a review against the assembly found that the pseudo-code's sums, minima, maxima and NaN branches are not the instructions', that the plane refresh had its normal reversed, and that the directional shadow volume and the volume cut, left out as ambiguous, are settled by the evidence.

## What Changes

- The tests follow the original's arithmetic: sums in its order, the processor's minimum and maximum (a NaN picks the second operand), sign-bit "apart" answers, the NaN side of each branch, the oriented-box overhangs compared as integer bit patterns.
- The plane refresh's normal is (v2 − v1) × (v0 − v1), normalised with a zero vector left zero; a polygon wound counter-clockwise from outside faces out.
- Rebasing a volume adds origin · normal to each offset (the volume seen from the origin), no longer described as a move by the origin.
- The segment slab test takes the segment's vector, as the original does.
- Polygon clipping keeps the first inner run from the last vertex's side, as the original walks it.
- New: cutting a convex volume by a plane, with the cuts chained into a cap on the plane.
- New: the directional shadow's caster volume (silhouette side planes extruded 10 000 cm toward the light, plus the faces turned away from it).
- The point-light cube cache's face mask is the original's exact side-plane test, read from the shipped table.

## Capabilities

### New Capabilities

- `intersection-tests`: the shared intersection and convex-volume routines.

### Modified Capabilities

- `rendering`: the caster face mask of the cached cube shadows.

## Impact

- `exa_physics::intersect` (corrected routines, `ConvexVolume::clip_by_plane`, `ConvexVolume::shadow_volume`, `ConvexVolume::relative_to`).
- `exa_render::cube_shadows::sphere_faces` (exact mask; the cube-face routine moves there from `intersect`, its only user).
- `docs/RENDERING.md` (the face-mask rule) and the `exa_physics` README.
- Evidence: REA (Ghidra) pseudo-code and assembly of the unit's 34 routines and of the vector helpers they call, the side-plane diagonal and face tables read from the executable, and the directional shadow's caller (unit light direction, w = 0). No corpus files.
- Out of scope: the unit's scene-graph bookkeeping (light-range globals, the weight-ordered node list, light-alias creation and spreading), the directional shadow pass and portal visibility that would call the volume routines, the body-pair meet's mesh-box tests (Exarust's parts have no mesh boxes yet), and the processor's approximate reciprocal and reciprocal square root (Exarust's are correctly rounded).

## Phase

Phase 3, bodies and physics: the shared intersection maths the body contacts, cloth and shadows build on.
