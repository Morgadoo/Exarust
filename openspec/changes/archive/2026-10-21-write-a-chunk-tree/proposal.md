# Proposal

## Why

The original stores every visited level inside a save by writing its live
scene back into a chunk tree. Exarust only reads chunk trees: nothing wrote
one, so the level export of the save game had no counterpart.

## What Changes

- The chunk framing the exporter uses: open (tag and zero size), close
  (patch the size), take an empty chunk back out.
- The walk that sorts the scene into scene nodes, props, placed objects and
  characters, numbers them (props after the last scene node's number), and
  finds each node's exported parent and data source.
- Every chunk the exporter writes, in its order: header, tile grid,
  material list (with the two texture-layer renaming switches), scene nodes
  (header masks, mesh streams and topology, lights, voxel lights, the
  `3D08` body, rigid bodies with weighted links, cloth markers, emitters,
  GI points, keyframe tracks), props and their part records, the local
  database with placed objects, characters (pose tree, AI home, motion
  state) and the environment.
- The save game's single-placement writer (flat pose, the attached
  objects' chunk taken out when empty).

## Capabilities

### New Capabilities

- `chunk-tree-export`: writing a scene as the original's level export does.

### Modified Capabilities

## Impact

- `exa_formats::rfc_write` (new): `ChunkWriter`, `Exporter`,
  `ExportScene` and its parts, `write_character_state`,
  `replace_extension`, `lowercase_name`, `weighted_point`.
- `docs/formats/RFC.md` (Writing chunk trees), `crates/exa_formats/README.md`.
- Evidence: REA (Ghidra 12.1.4) on the original's level exporter and its
  helpers (the walk, numbering, node, mesh, light, body, prop, object and
  character writers, the database writers, the name helpers); constants
  read from the executable's data (the identity hand placement,
  `_Default`, `.rfi`/`_rfi`, 1.0). No corpus file is needed: tests write
  synthetic scenes and read them back with Exarust's reader.
- Out of scope: building an `ExportScene` from Exarust's live world, the
  save package that would carry it, the material hooks and state syncs the
  exporter calls before writing (they change no bytes), the string and
  database builders behind `DBCD`, `DBDB` and `CECDDB00` (their output is
  taken as given), and an `exa-tool` rewrite command (the reader keeps too
  little of a file to write it back faithfully).

## Phase

Phase 7, save games: the level export a save stores visited levels with.
