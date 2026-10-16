## 1. Reverse engineering

- [x] 1.1 REA: render targets of the item and character paint jobs and the sRGB-encoding state while painting
- [x] 1.2 REA: load flags of shade maps, masks, item normals and every procedural material's library textures; the palette colour's path to the paint programs
- [x] 1.3 REA: the run-time painted material and the scene shader's diffuse term
- [x] 1.4 Research notes in `context/re/painter-colour-space.md` (git-ignored)

## 2. Painter and upload

- [x] 2.1 `exa_assets::paint`: `Encoding` and the colour, surface and normal maps' encodings, with the chain in the module documentation
- [x] 2.2 `exa_assets::model`: the upload takes the encoding; sRGB mips average colour in linear light, alpha and linear maps as stored
- [x] 2.3 Unit tests: palette colour 1 on a mid-grey shade map (plain finish texel 160 sampled 0.3515, rough cloth texel 96 sampled 0.117, the original's cloth program giving the same) and the maps' formats; linear-light mips

## 3. Verification

- [x] 3.1 Corpus (skipped when `EXANIMA_DIR` is unset): `cargo exa paint braies_a braies__aa`, `doublet_a doublet_aa` and `coif_a coif____aa`, comparing mip 4 averaged as stored and in linear light (braies +11 %, doublet +8 %, mail +49 %)
- [x] 3.2 Screenshots through `gpu_run.sh`: `exarust play exanima01 --frames 900` and `exarust character --outfit doublet_a,trouser_a --frames 120`, before and after, compared with the original's screenshots
- [x] 3.3 Remove probes and dumps from the tracked tree

## 4. Documentation

- [x] 4.1 `docs/formats/OTHER.md`: Colour space paragraph
- [x] 4.2 `docs/PLAN.md`: Phase 3 done line
- [x] 4.3 `openspec validate --all --strict`
- [x] 4.4 `make check`
