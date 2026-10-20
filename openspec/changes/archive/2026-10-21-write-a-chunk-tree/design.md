# Design

Format page: [`docs/formats/RFC.md`](../../../docs/formats/RFC.md), Writing
chunk trees.

## Input model

The original walks its live scene objects. Exarust has no such objects in
`exa_formats`, so the exporter takes a flat list of nodes (`SceneNode`),
each naming its parent; children keep the list's order. A node carries what
the exporter reads from the live object: its flags (with a possibly stale
exported bit), second flag word, local and world matrices, bounds, the tail
word, its export number from before the walk, the node it shares data with,
its body, attachment, keyframe controllers, and for characters and placed
objects their state. Values are in file units (cm), so the writer encodes
them as they are; the reader's conversions to metres stay in the reader.

## Faithful quirks kept

- Props are numbered from the last scene node's number, so the first prop
  and the last scene node share a number.
- The material list's count includes materials the scene's library does not
  hold, which are then not written.
- Characters are not renumbered; references to them use the number they
  held.
- A stale exported bit on a node the walk never reaches still counts as
  exported (parent, data source).
- Link records end at the first link marked `0x400000` and the count is
  patched to those written.

## Guards (where the original would misbehave)

- An emitter kind other than `0x3DD0E002`/`0x3DD0E004`: the original closes
  a chunk it never opened; Exarust returns an error.
- A texture layer of length 0 (the original would loop forever) or one that
  runs past the payload ends the layer walk.
- Missing data the original would dereference (a mesh node without a mesh,
  a saved character without its AI, a pose that is written but absent) and
  lists shorter than their counts are errors.

## Open questions

- The meaning of the `3D08` body (a node number and a word; guess: an aim
  target), the AI skip byte, AI mode 1 and the record kind `0x1000`: named
  by their effect on the output only. Answer with REA on the readers of
  those fields when the save system needs them.
- Which sub-chunk a mesh keeps whole for writing back: not traced; the
  writer takes it as given. REA on the mesh reader's store of it.
- The library lookup's case rule is taken from the material manager's
  lowercasing of names (inferred).

## Tests

Byte-exact checks of the framing, grid, node header masks, mesh stream
count, index widths, light and track records, link cut-off, part records,
database and character chunks; round trips through `rfc::Scene::parse` and
`LevelCharacters::parse` for meshes, lights, voxel lights, bodies, cloth,
emitters, tracks, GI points, props, placed objects and characters.
