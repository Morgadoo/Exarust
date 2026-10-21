# Proposal

## Why

The original joins one mesh part onto another in an assembly routine that
works on editable channels, and its after-load init sets a per-mesh render
word. The branch's first attempt at both ported the shape but not the
rules: missing triangle flags joined as 0, a side without texture
coordinates was zero-filled, the part's GI points were appended, and the
render word's `0x800` bit was taken for a main-pass cull switch that
drew a pipeline variant. Reading the machine code shows the bit only
selects front or back face culling in the shadow depth pass, and that the
stored vertex colours only feed dirt modes Exarust does not shade.

Phase 5 (Visual and audio fidelity), the mesh unit milestone started by
`smooth-the-meshes`.

## What Changes

- `Mesh::merge` joins the streams as the assembly does: flags missing on a
  side join as 1 and edge bits as 7, a side without texture coordinates
  indexes element 0 of the joined stream, the destination's missing
  channels are made padded with white colours or zero weights, and the
  GI points stay with the node.
- `Mesh::shadow_cull_front` names the render word's `0x800` bit as the
  shadow pass's cull selector; no model node carries it and no material
  variant is drawn for it.
- The stored RGBA8 colour stream stays parsed but is not uploaded as a
  vertex attribute; the format notes record the counts (2 708 coloured
  meshes, 859 drawing a dirt-mode material, none the wound-blood mode).
- `exa-tool corpus colours` counts the shipped vertex colours per pack.
- A mesh without faces warns once per entry and node.

## Out of scope

- Calling `Mesh::merge` from a loader: no Exarust environment assembles
  mesh parts at run time yet, and the caller must bring both meshes into
  one node space.
- The shadow pass's culling (Exarust draws no mesh with the bit).
- Dirt modes 1 to 3, which read the vertex colours.
- The original's editable-channel bookkeeping (channel keys, GPU buffer
  patching) and the smoothing key channel rebuild.

## Evidence

REA (Ghidra 12.1.4) on the original's mesh assembly, after-load init and
both shadow-pass draw routines, read against their assembly. The
power-of-two resample wrapper and kernel normaliser of the mip unit were
re-read against their assembly and match the code on main. Real data:
`exa-tool corpus colours` over every mesh pack of the installation.
