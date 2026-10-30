# Proposal

## Why

Third pass over the level exporter (docs/PLAN.md Phase 7, saves: the
level section a save stores for a visited level; the earlier
`write-a-chunk-tree` change wrote the exporter itself). The exporter
lacked the scene's preparation calls, the option that skips the mesh-part
pass and the character condition words, and Exarust had no way to turn a
loaded level back into the exporter's scene, which a save needs.

Evidence: the original's assembly for the collection walk (the part pass
from the last part down, skipped by option bit 3 before the material
loop), the node dispatcher (the attachment's method under node flag
0x10000), the characters writer (every character not of class 1 refreshes
its record), the condition-word routine (two shares through CVTSS2SI, the
second only above zero, the fourth word through a helper that returns its
argument) and the light body writer (amplitude, speed, emissive), through
REA and the objdump listing. The level conversion was checked against two
whole shipped levels on the installed game.

## What Changes

- `exa_formats::rfc_write`: `ExportHooks` and `Exporter::export_with`
  (part, attachment and character preparation where the original calls
  them), `ExportOptions::skip_part_prepare`, `character_condition_words`,
  the light animation as the reader's `LightAnimation`, a camera node with
  no target storing 0, cloth particles in the 16-byte stride, the name
  list helpers, and a mesh node that shares another's data taking its
  master's parts.
- `exa_formats::rfc_level` (new): `LevelRaw::scan` for the records the
  reader drops (material records, object database and strings,
  environment, character database), and `level_scene` converting nodes,
  meshes, lights, bodies, props with their part records, placed objects,
  characters with their pose trees, and the grid back into the exporter's
  scene in centimetres.
- `exa_game::level_transition::exported_level_section`: the save's one
  call (convert, write with the save switches and the story settings
  gate).

## Out of scope

- Wiring `exported_level_section` into the live save package: the session
  does not keep the loaded section, and storing it would make the next
  load prefer the converted section over the archive.
- Rebuilding normal streams, the stored stream kinds the reader does not
  decode, a free key track's mode word, the record kind word of a
  character placement and the GI lattice spacing (a measured 40 cm).
- The semantics of the preparation hooks (the original's virtual methods
  are untraced; the caller supplies them).
