## Why

Every character of the original has its own body shape: the character
builder weighs the targets of the race's morph set by the record's
physique values, height, age, feral level and sex, morphs the body parts
and the apparel fitted to them, and moves the parts' origins so the
skeleton, the skin and the physical body follow. Exarust gives every
human the same body: zombies, guards and the stout and the slight all
look alike, and a short character keeps a tall skeleton.

Phase 3 (Bodies), milestone M3 "standing man" (MS5 "every character its
own" of the master plan): master-plan change M18 of lane B (animation),
after M39 (whole character records) and the skinning in `exa_assets`.

## What Changes

- Morph sets (`0xEDC0C589` after a mesh body, `humanmorph.rfc` and
  `ancientmorph.rfc`): landmarks (the 18 parts' origins), the base
  positions of the mesh's position stream and the targets' landmark and
  position offsets, in metres; a mesh keeps its position-stream count and
  smoothing keys; a set that does not decode keeps its error.
- Physique weights from a record as the builder sets them: the physique
  values kept in 0…1 and adjusted by height, the picker's four corners,
  the feral level, female (targets 5 and 6), age (target 4 and the shares
  of targets 0 and 4), height (target 10).
- The transfer: a body part or garment piece takes from the body, per
  position element, the barycentric blend of the targets' offsets at the
  nearest surface within 10 cm, less the offset of the landmark nearest
  the piece's origin, in the piece's frame; elements farther out take the
  inverse-distance mean of their set neighbours, pass after pass. A
  garment piece is carried over from the frame of the base body part its
  node names.
- Morphing per character, before skinning: positions = base + Σ w·offset
  (weights below ≈1.2e-7 skipped), normals from the new shape, boxes
  refitted, each part's origin moved with its landmark and its
  translation recomputed under its moved parent; the body's parts are
  skinned on the morphed shape and joints, garments from the unmorphed
  body. Variants are cached by race and physique; a physique that weighs
  nothing, and races without a morph body, keep the shared model.
- Placed characters take their records' physique; the player, the
  duellist and the character view keep the model's body; `--physique`
  morphs the character view's body or the player's.
- `exa-tool morphs [--records]`: the sets, and every placed record's
  weights.
- Docs: RFC.md (Scene nodes, Morph sets), DEBUGGING.md (`--physique`,
  Physique), PLAN.md (Phase 1 table, Phase 3), crate READMEs.

Evidence:
- REA on `Exanima.exe` 0.9.5 and its disassembly: the race loader's set
  load and the transfer to the base parts, the set reader, the physique
  pass and the picker (every constant read from `.rdata`/`.data`), the
  part apply, the joint walk and the factory's pivot table, the order of
  morph, limb bodies, motion binder and part list in the builder, the
  apparel placement on the base body parts, the dressed mesh's transfer,
  apply and skinning with the morphed pivots, the bounds refit, the
  sphere query and its 10 cm radius (`context/re/master-animation.md`
  §10 and "M18 findings").
- Corpus: both morph files decode (18 landmarks, 2286 positions = the
  mesh's position stream, 11 targets), every `humanbase.rfc` element
  finds the body within 10 cm and each part's nearest landmark is its
  own; weights of the 477 placed records (126 with a morph body, none
  neutral).
- In game: light character-view shots of the neutral, a tall slight and a
  short broad physique, dressed and undressed.

Out of scope:
- refitting the limb spheres to the morphed meshes (C20; Exarust's part
  boxes already come from the morphed meshes);
- the original's skin-weight transfer to garments through the same
  mapping (Exarust keeps its nearest-vertex apparel weights), race flag 4's
  auxiliary data, the old numbered-target set layout (unused);
- the character-creation sliders and the turning character view (M19);
- hair, skin tones and the age darkening of skin.

## Capabilities

### New Capabilities
- `physique`: morph sets, physique weights, the transfer, morphed bodies,
  joints and apparel.

### Modified Capabilities
- `asset-inspection-cli`: `exa-tool morphs`.

## Impact

- `exa_formats` (`morph`: `MorphSet`, `MorphTarget`, `Physique`,
  `MorphBody`, `PartMorph`, `morph_scene`, `morph_mesh`, `bounds`,
  `model_frames`; `rfc`: `MORPH`, `Node::morph`, `Mesh::position_count`,
  `Mesh::smoothing`, `Mesh::recompute_normals`)
- `exa_assets` (`morph`: `BodyMorph`, `parse_physique`,
  `ModelLoader::{morph_body, morphed_model, morphed_job,
  loaded_morphed_model, morphed_key}`; `model`: `ModelJob` morphs before
  building, skins garments before and bodies after; a test mesh literal in
  `skin`)
- `exa_character` (`equip_outfits` fits garments as variants morphed with
  the wearer's `BodyMorph`)
- `exarust` (placed characters, player and character view; `--physique`),
  `exa_tool` (`morphs`)
- `docs/formats/RFC.md`, `docs/DEBUGGING.md`, `docs/PLAN.md`,
  `crates/exa_formats/README.md`, `crates/exa_assets/README.md`,
  `crates/exa_character/README.md`, `crates/exa_tool/README.md`,
  `crates/exa_render/README.md`
