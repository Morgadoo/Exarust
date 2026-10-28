# Proposal

## Why

The voxel GI read its light texture at each slice's boundary, compared the
point lights' cube shadows against d/R + 0.01 without the original's bias
factor, and gave every surface the same sample origin. The shipped
`gi_inject_v/f` sources and the manager's light-uniform builder show the
original reads each voxel's own slice centre, scales the cube compare by
(bias + 0.94)/far in double precision, and lifts a surface's sample by half
a voxel except on the terrain and grass paths. A DeepSeek pass (lane
`vxgi3`) ported these; the review checked them against the assembly, added
a test that pins the CPU's GI uniform structs to the shader's layout, took
the per-object offset's last steps in double as the original does, and
labelled the unranged-light lookup as a guess.

## What Changes

- The propagation reads layer L at r = (L + 0.5)/depth and measures light
  distances at (slice + 0.5)/width.
- The point-light uniform is three vec4s (position and 1/R; colour and the
  cube depth scale ((bias + 0.94)/far) × 5120; shaping); the cube slot moves
  to its own array. The bias is the retail shadow detail 5 plus the light
  record's quality offset, capped at 6, × 0.005, or 0.025 above 5.
- Surfaces sample half a voxel up; terrain and grass read the voxel's
  bottom. `GiBlockLayout::sample_transform`/`sample_at` keep the
  original's per-object offset and scale.
- A node's flags2 `0x10` reaches its entity as `GiCentredMesh`.
- Tested manager rules: shape program names, shape quad and shared
  uniforms, margin quad.

## Capabilities

### New Capabilities

### Modified Capabilities

- `rendering`: GI cube shadows; GI cube shadow bias; GI slice-centre sampling; GI surface sample offset.

## Impact

- `exa_render::gi` (`gi.rs`, `gi.wesl`), `exa_world::gi_volume`,
  `exa_world` light bundle and terrain GI, `exa_assets::model`
  (`ModelNode::gi_centred`).
- `docs/RENDERING.md` (Voxel GI), crate READMEs.
- Evidence: the shipped `gi_inject_v/g/f` sources (quad r −0.5/depth, the
  layer offset subtracted, `ZTrn`); the light-uniform builder's
  CVTSS2SD/ADDSD 0.94/DIVSD far/MULSD 5120/CVTSD2SS; the cube texture
  acquisition (detail + offset capped at 6, q·0.005 in double, 0.025 above
  5; the record's unranged far plane 3.4e38); the per-object lookup (the
  floored 48 m block lookup, +0.5 unless the caller's flag, the X/Z offset
  in double); its four callers (flag 1 for terrain and grass, `seta` of
  flags2 & 0x800000 for lit materials); the mesh splat's flags2 0x10 test.
- Uniform: `GiLight` shrinks from 64 to 48 bytes and a 3 KiB slot array
  is added; the block stays under 64 KiB and its size is pinned against
  naga's layout of `gi.wesl`.

## Out of scope

Nine resident blocks and the margin blit pass, the shapes' fragment bodies
and uniforms, the lit-material flag `0x800000` exception, the thin GI
mesh byte, the top-slice flag, wiring the dynamic update, and drawing an
unranged light's cube faces against the 3.4e38 far plane.

## Phase

Phase 2, static world: the voxel GI (lighting milestone).
