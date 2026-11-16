## Why

Roadmap task B4 (lane `morph2`): the morphing unit's open rows. Read
block by block against the assembly, the live apply, the transfer and the
morph body's choice differed from the original in order, not in kind:
the apply summed the targets' offsets before adding them to the base, the
transfer walked an element's neighbours first face first, the morphed
boxes used a refit of their own, and the morph body was the first node
carrying a set rather than the first root's.

## Evidence

REA (Exanima.exe 0.9.5), read in the instructions:

- The per-mesh apply and the origin read (the character builder's two
  callers): the blob's base positions copied over the position stream,
  then per target whose weight passes `andps`/`comiss` against ≈1.2e-7
  (`jp` applies a NaN) the origin += lift·w (vector multiply-add) and each
  position += delta·w (scale, then add in place); then the mesh refit.
- The transfer's diffusion and both skin passes read each element's
  corners from the mesh's incidence (its start table and corner list),
  the neighbour through the next-corner table {1, 2, 0}; the load leaves
  that incidence last face first, split into smoothing-key runs.
- The race loader takes the morph file's first root's set directly
  (`cmp [root+set], 0`); only without one does it call the fallback that
  scans the scene's nodes and then the numbered target meshes.
- The slider-driven controller apply, its base rewrite and its attached
  piece's apply are called only from the character-morph slider
  callbacks, whose records no code reads.

## What changes

- `exa_formats::morph`: `PartMorph::apply` / `origin_at` add each target
  in turn to the base; `morph_mesh` takes the morphed positions;
  `morph_scene` refits with `rfc::Mesh::bounds` (the old `morph::bounds`
  is removed); `element_neighbours` walks the load's incidence
  (`Mesh::element_incidence`); `MorphBody::of_scene` takes the first
  root's set; `MorphBody::transfer_with` compares walks.

## Impact

Morphed positions and joints change by float rounding (lockstep traces of
morphed bodies change in the last bits). Over the 4 001 garment pieces
on the human body the walk changes the morph offsets of 12 395 elements by
at most 4.5e-8 m; the transferred skin records of 77 089 (human) and
75 770 (ancient) elements change, nearly all in the last bits, 555 and
131 in their joints. Character creator: unchanged (no slider path).
