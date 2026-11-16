## Why

Roadmap task B4 (lane `skinning`): the skinning unit's partial rows and the
morph transfer's skin-weight branch the wiremorph lane left open (round
215). A garment piece whose render word carries `0x200000` takes the race
body's skin weights through the morph transfer in the original; Exarust
built every garment's weights from its seams. The body's skin records the
branch reads, and the merge that builds them, had no port. The skin pass
handed group centres down its own recursion without the builder's
sixteen-frame stack, its overflow error or its eight-centre frames
(finding F244).

## Evidence

REA (Exanima.exe 0.9.5), read in the instructions where constants, order
or branches matter:

- The transfer's skin branch: the stream lookup or append (0x4000001 /
  0x2a41000), the corner merge at the barycentric weights through the
  four-slot merge (scale compare `comiss` with ≈1.2e-7, NaN merges), the
  neighbour fill (`sqrtss` then `divss` 1/d, not divided by the total,
  `comiss` 0 > total to set), the normalisation (joint `<= 0` ends the
  slots, negative weight to 0, NaN summed), the list of corners of the
  triangles with a `0xC00000` flag bit (corners 0, 1, 2), the three
  relaxation passes (halve, merge neighbours at 0.5/d, read before the
  pass, normalise without the sign test). Constants 0, 1, 0.5 and the next
  corner table {1, 2, 0} read from the data.
- The race loader: the merged-mesh walk (class `0x89`, render word
  `& 0xC00000 == 0x400000`, which the loader's earlier walk sets on every
  part not named `#` or `_`), the mesh assembly in the first root's frame,
  the weld, and the nearest-position copy (`comiss best, d`, strict, first
  of equals, from 3.4e38); run only for a look word with bit `0x4`.
- The garment build: the transfer runs before the skin builder's per-node
  pass, whose prepare keeps the 0x1000 stream it finds (empty push).
- The group stack: frames of 100 bytes from the builder's 100th byte, the
  `< 16` push test with the "Skinning group stack overflow." error, the
  unconditional pop after the children, the empty push without a test,
  and the reader at the depth's frame.

## What changes

- `exa_formats::morph`: `PartMorph::skin`, `MorphBody::with_skin`,
  `merge_skin`, `normalise_skin`, `nearest_skin_records`, the transfer's
  skin branch for pieces with `BODY_SKIN`.
- `exa_assets::body_skin`: the merged, welded race body and its records,
  attached by `ModelLoader::morph_body`; the model build keeps the
  transferred weights (`morph::given_skins`, `scene_skins`' `given`); a
  garment on a race with a morph body is transferred at any physique (a
  skin-only variant below the smallest weight).
- `exa_assets::skin::GroupStack` laid out as the original's frames and
  driven by the model build's skin pass.
- The test-only controller fragments the live port replaces are removed
  (`morph_controller`'s skin records, stream registration, target list and
  nearest-record scan; its target list had the corners in the wrong order).

## Impact

Every garment piece with render bit `0x200000` (2 694 of 4 001 in
`Characters.rpk`) on a human or an ancient changes weights: the dominant
bone changes on 27 % (human) and 28 % (ancient) of their vertices. The
records are built once per race and a garment's once per race and model;
nothing runs per frame. The GPU scenes owed: humans and ancients of several
physiques and ages, dressed and undressed, walking and bending.
