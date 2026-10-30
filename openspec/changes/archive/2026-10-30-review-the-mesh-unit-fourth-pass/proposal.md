# Proposal

## Why

Phase 5 (visual fidelity), texture and surface milestones: every model's
GPU geometry comes from the mesh unit's build, and the unit's face tests
are the mesh class's collision queries. A fourth pass on `RF_Mesh` and
`TMesh` corrected the swept-sphere distance, applied the material's
texture matrix to the uploaded coordinates and added the colour-channel
helper. A review against the assembly found that the swept-sphere walk
keeps the sweep's travel and the best hit in one field, so every accepted
hit shortens the sweep the later faces are tested with; the port kept the
two apart.

## What Changes

- The swept-sphere face test measures an accepted hit from the sweep's
  start (`closest·d − start·d − √(r² − p²)`) and takes a single travel
  value that is also the best hit; the mesh walk passes the running best.
- The mesh build rewrites a group's uploaded `UV_0` through the shading
  record's 12-float texture matrix when its extra flags carry `0x40`
  (`(u·m0 + v·m3) + m9`, `(u·m1 + v·m4) + m10`); the split tangents still
  come from the stored channel. Wired into model assembly.
- `EditMesh::add_colour_channel`: the first stream's id with the
  shared-channel bit, the colour format word, the first stream's channel
  and count, opaque white unless bytes are supplied. No caller yet.

## Capabilities

### Modified Capabilities

- `mesh-geometry`: swept-sphere travel (origin and shortened sweep),
  material texture matrix.
- `mesh-editing`: colour channel creation.

## Evidence

objdump listing of `Exanima.exe`: the swept-sphere face walk (its query
record's travel/best field read for the plane approach and the grazing
end point and overwritten by an accepted hit; the start-relative distance;
the parity-jump compares), the point-plus-scaled-direction and vector
difference helpers it calls; the GPU build's UV-key branch on the
material's extra flag `0x40` and the channel copy's matrix arithmetic
(its sum order), ahead of the tangent split that reads the stored
channel; the colour-channel helper (id or `0x04000000`, format
`0x02B40100`, channel and count from the first element, `0xFF` fill).

## Out of scope

- The face queries' game callers (collision stays rapier) and their
  callback push-out forms.
- Handing a new colour element to every instance (Exarust shares an
  instance's model), the stored vertex-colour upload, morph-delta buffers.
- The after-load static cast clear, cached-centre lowering and modifier
  notification.
