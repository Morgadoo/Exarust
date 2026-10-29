# Proposal

## Why

docs/PLAN.md, Phase 2 (static world), the gridenv paragraphs: the round-34
gridenv4 lane ported the plant ground query's diagonal normal correction,
the grass draw, sun-shadow and omni-shadow hooks' per-chunk tests and the
importer finalise pass's two node visibility walks. The review checked each
against the assembly and found the sun-shadow test reading the wrong node
word.

## What Changes

- `vegetation::ground_with_normal`: the corner's right neighbour as origin,
  the `(−x + z)·0.05 − cross.y·0.0025` test against 1 (a NaN takes the far
  diagonal), the `±0.0025·dot` correction with the engine's
  `(x + y) + z` dot. A tested rule only: the original's caller discards it.
- `vegetation::chunk_drawn`, `chunk_sun_shadow`, `chunk_omni_shadow`: the
  hooks' block, mask, skip and instance tests. The sun-shadow bit is the top
  bit of the chunk node's mask word (the review moved it off the skip word).
- `Scene::apply_visibility_flags`: `0x100000` hides light nodes of a tree,
  `0x200000` hides light and fire nodes once the fade value is above the
  f32 0.12 (unordered compare: NaN and exactly 0.12 leave them shown).

## Evidence

REA/objdump on the plant ground helper (truncating conversions, 241-vertex
stride, vector subtract and SSE cross helpers, `JP`/`JA` to the far
diagonal, dot sums), its caller in the per-block generator (the result is
stored to a stack slot never read again), the main draw, sun-shadow and
omni-shadow hooks (block `0x80`, `0x10000|0x20000`, node mask `&` view mask,
node word `& 4`, node mask `& 0x80000000`), and the two recursive
visibility walks with the finalise and content-stream call sites.

## Out of scope

Wiring: the plant ground result is dead in the original, the hooks' block
and node words have no Exarust equivalent (Bevy visibility and shadow flags
draw the chunks), and a whole-level load has no fade value to pass the
visibility pass. Wind, the mask-texture colour lookup, the room-mode-off
path and the omni room-ceiling shadow shell remain unported.
