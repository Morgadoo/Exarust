## 1. Research

- [x] 1.1 RB42 in REA and `exe.asm`: the set reader and wrapper, the physique pass and picker with every constant, the race loader's transfer to the base parts, the transfer and its mapping (sphere query radius, barycentric solve, landmark search, neighbour fill), the part apply and bounds refit, the joint walk and the pivot table, the builder's order (morph, limb bodies, motion binder, part list), the apparel placement on the base parts, the dressed mesh's transfer, apply and skinning; findings in `context/re/master-animation.md` ("M18 findings")

## 2. Decoding and rules (`exa_formats`)

- [x] 2.1 `rfc`: `MORPH`, `Node::morph` after a mesh body (decoded with the mesh's position-stream count, errors kept), `Mesh::position_count`, `Mesh::smoothing`, `Mesh::recompute_normals` (never panics on stray indices or keys)
- [x] 2.2 `morph`: `MorphSet::parse` (landmarks, base, targets in metres; truncated, negative and overflowing counts are errors), `Physique` (`of_record` with the builder's clamps, `NEUTRAL`, `weights` with the picker only past three targets), `MIN_WEIGHT`, `morphs`
- [x] 2.3 `morph`: frames (`place`, `unplace`, `unturn`, `compose`, `model_frames` with loops cut), `MorphBody` (`of_scene`, `new`, `nearest_landmark`, a 10 cm grid for the nearest surface), `MorphBody::transfer` (barycentric blend less the nearest landmark's offset, into the piece's frame; neighbour fill pass after pass), `PartMorph::offsets`, `morph_mesh`, `bounds`, `morph_scene` (meshes, normals, bounds, then translations from each root down, a node without a share ending the walk)

## 3. Variants (`exa_assets`, `exa_character`, `exarust`)

- [x] 3.1 `exa_assets::morph`: `BodyMorph` (race stem, physique, cache key), `parse_physique`, `ModelLoader::{morph_body, morphed_key, morphed_model, loaded_morphed_model, morphed_job}`; transfers cached per race and model (garment pieces from their bone's base part frame), variants by the physique; neutral weights and races without a morph body return the shared model
- [x] 3.2 `exa_assets::model`: `ModelJob` morphs the scene before building, skins garments before the morph and body parts after it, caches the variant under its key and its shares, and never caches a variant's positions as the body's apparel reference
- [x] 3.3 `exa_character::equip_outfits`: garments fitted as variants morphed with the wearer's `BodyMorph` (async build keyed by the variant's key)
- [x] 3.4 `exarust`: placed characters take their records' physique (the build time logged); the player keeps the model's body unless `--physique`; the duellist keeps it; the character view takes `--physique`

## 4. Tool

- [x] 4.1 `exa-tool morphs [--records]`: the sets (node, place, landmarks, each target's reach and moved joints in cm) and every placed record's weights with the counts and each weight's range

## 5. Tests

- [x] 5.1 `morph`: a synthetic set decodes in metres and positions blend; malformed sets are errors; the smallest weight 1.192e-7; weights pinned: neutral all 0, a new record w3 = 1, (0.8, 0.3, h ½, a ½, m ½, female) and (0.2, 0.6, h −0.4) by number, feral past the corner (w0 1.5, w4 ½), clamps and NaN, small sets
- [x] 5.2 `morph`: the transfer on a square body (origin with landmark 0, on-body offsets 1 cm, the neighbour fill, the unreached −1 cm, the barycentric 0.3 of corner 0 = 3 cm, weights ½ and 1, tiny weights skipped), a turned frame, normals after a tilt, bounds with the least extent, frames composed and loops cut, `morph_scene` moving the root 10 cm and a turned child's translation to (0.1, −0.03, 0) m, a plain node ending the walk
- [x] 5.3 Corpus (skipped without `EXANIMA_DIR`): both morph files decode (18, 2286 = the mesh's position count, 11; hip landmark; height drops 11.20/6.21/13.59 cm, feet 0); every `humanbase.rfc` element finds the body within 10 cm, each part's nearest landmark is its own, height moves the pelvis 11.196 cm and the right thigh 11.920 cm down and 1.499 cm in, the body stands where the pelvis does, unturned
- [x] 5.4 `exa_assets` corpus: the neutral body is the shared model; physique (1, 1) height 1 drops the pelvis 11.2 cm, the knee 5.7 cm nearer, boxes and vertices changed, the variant cached; `trouser_a` moves with unchanged joints and weights; a skeleton keeps its model; key and flag parsing

## 6. Verification

- [x] 6.1 `exa-tool morphs` and `exa-tool morphs --records` against the installation: 2 files, 18/2286/11, target 1 11.61 cm, target 10 13.54 cm; 477 placed, 126 with a set, 0 neutral
- [x] 6.2 In game, light runs through `gpu_run.sh --gpu 4000` (`--no-gi --no-supersample --size 960x540`): the character view without `--physique` matches main's binary within run-to-run noise (110–188 px against 126 between two runs of the branch), `--physique 0,1,-1` and `1,1,1` change 18 740 and 18 489 px (taller and slighter; shorter and broader), dressed in `leathjac_a,trouser_a,boots_b` the garments follow; no panic or validation error
- [x] 6.3 Remove probes and scratch files from the tracked tree (the transfer and apparel probes were temporary and are gone)

## 7. Documentation

- [x] 7.1 `docs/formats/RFC.md` (Scene nodes, Morph sets), `docs/DEBUGGING.md` (`--physique`, Physique), `docs/PLAN.md` (Phase 1 table, Phase 3), `crates/exa_formats/README.md`, `crates/exa_assets/README.md`, `crates/exa_character/README.md`, `crates/exa_tool/README.md`, `crates/exa_render/README.md`, the launcher's option list
- [x] 7.2 `openspec validate --all --strict`
- [x] 7.3 `make check` equivalent on the `play` profile: `cargo fmt`, `cargo clippy --profile play --workspace --all-targets -D warnings`, `cargo test --profile play --workspace` with `EXANIMA_DIR`
