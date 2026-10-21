# Proposal

## Why

Phase 5 (visual fidelity), texture and surface milestones: generated mip
levels and generated normals and tangents decide how every surface looks.
A first pass rebuilt the original's resampling unit (`RFP_Mipmaps_Kaiser`)
and the mesh unit's normal, tangent, bounds and regroup helpers (`TMesh`,
`RF_Mesh`). A review against the instructions found rules it missed or
misstated: data images filter with a second, uniform bank; the chain wraps
or clamps each axis by the image's own flags; the Kaiser sum is double
precision; the corner weight is an arc-cosine approximation, not a sine;
faces with a fallback flag shape a smoothing group only alone; the
tangent sign takes the unprojected tangent; the bounds floors and the
barycentric helper's plane projection.

## What Changes

- The generated chain picks the uniform bank (`2^L` taps of `2^−L`) for
  data words and the Kaiser bank for colour, wraps or clamps each axis by
  the image header's `0x10000`/`0x20000`, and renormalises only three- and
  four-channel images flagged `0x4000000`, with the engine's normaliser.
- The Kaiser table sums its weights in f64 and divides each by that sum;
  the sinc argument is `u·π·c`; the Bessel loop stops on an ordered compare.
- Chains start at a requested first level, keep their final 1×1 level and
  floor uploaded sides at one texel.
- The resample to an arbitrary size takes the chain's level for a
  power-of-two ratio, picks its filtering space by word and size, and keeps
  a zero integer ratio when it magnifies.
- Mesh normals: unit face normals weighted by the corner-angle
  approximation, faces flagged `0x10000` counting only alone. Tangents,
  bounds, barycentric coordinates, index type and the material regroup as
  the original computes them. The sub-image blits, sharpen kernel, kernel
  normaliser, bytes per pixel and icon canvas placement.

## Capabilities

### New Capabilities

- `mesh-geometry`: generated normals, tangents, bounds, barycentric
  coordinates, index type and the material regroup of a static mesh.

### Modified Capabilities

- `rendering`: the Kaiser kernel, the axis pass, the chain and texture
  mips; new requirements for the bank choice, renormalisation and the
  resample to size.

## Evidence

REA on `Exanima.exe` 0.9.5: the unit's two bank builders, the kernel and
Bessel/sinc helpers, the four axis passes and their accumulate callbacks,
the chain driver and the texture loader's call into it, the window kernel
builders and the resample driver, the vector normaliser, the blits, the
canvas copy, the sharpen kernel; the mesh unit's face-plane record, the
corner-angle helper, the normal builder, the tangent splitter, the bounds
refit and its box helper, the barycentric helper, the GPU build's index
type and the regroup. Corpus (`EXANIMA_DIR`): all 8839 shipped images
convert (151 generate a chain: 33 with the uniform bank, 58 clamped on an
axis, 30 renormalised); 16 640 shipped meshes get finite unit normals
(130 415 faces carry the fallback flag).

## Out of scope

- Filtering in the format's native channel count (Exarust always decodes
  to RGBA8 first; the per-channel arithmetic is the same).
- The generic 12-thread float-image driver, the sharpen convolution pass
  and the icon manager that would call the resample and canvas helpers.
- The per-face normal channel of terrain and cloth, the modifier pass, the
  character mesh copy and the item/grid/character mesh merge.
- Byte index buffers (Bevy has none; groups of up to 255 vertices use u16).
