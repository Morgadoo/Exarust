# Proposal

## Why

Phase 5 (visual fidelity), texture and surface milestones: every model in
the game gets its normals from the mesh unit's builder, and the item
factory, grid builder and character generator assemble their meshes with
the unit's merge. A third pass on `RF_Mesh` and `TMesh` added the editable
assembly, the regroup, the deep copy, the face record and its queries, and
the incidence order of the normal sums. A review against the assembly
found the sums missed the original's per-element smoothing-key partition,
the merge used a different float order for the relative frame and the
placement and copied the normal stream the original skips, and the face
tests rejected NaN where the original's compares let it through.

## What Changes

- Normal sums walk each position element's corners last face first, last
  corner first, split into smoothing-key runs by the original's in-place
  (unstable) swap partition. Face normals are computed once per face.
- The cloth pass's unweighted per-group face-normal average.
- The face record (unit normal, plane, corners, first-edge square,
  reciprocal squared area) and its queries: closest point, barycentric
  weights, strict and inclusive ray hits, sphere push-out, reach test and
  swept-sphere first touch, with the original's NaN behaviour.
- The editable-form assembly: groups join by material, channels by low
  key, streams by format word (the normal stream skipped), the position
  stream placed by the rigid `dst⁻¹ · src` in the original's float order,
  gaps filled with 0 or 255, flags 1, edges 7; the material regroup that
  permutes every channel and per-face array; the deep copy.

## Capabilities

### Modified Capabilities

- `mesh-geometry`: normal sum order, cloth normals, face record queries.
- `mesh-editing`: editable part assembly, regroup and copy.

## Evidence

REA/objdump on `Exanima.exe`: the angle-weighted and cloth normal
builders, the circular incidence lists and their two smoothing-key
partitions (by key and by flag-mask closure), the closest-point helper,
the ray, inclusive-ray, reach, sphere and swept-sphere face tests (their
parity-jump compare patterns), the rigid inverse, the matrix product and
the point placement helpers, and the 5.6 KB merge (its stream skip on low
key 2, its 0/255 gap fill and its flag and edge fills). Corpus
(`EXANIMA_DIR`): every shipped mesh still gets finite unit normals.

## Out of scope

- Wiring the editable assembly into the item, grid and character builders
  (they keep one Bevy mesh per part); the face queries have no game caller
  (collision stays rapier).
- The callback push-out forms of the swept tests, the modifier pass, the
  material UV matrix, morph-delta buffers and byte index buffers.
