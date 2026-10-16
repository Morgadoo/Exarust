## 1. Formats

- [x] 1.1 `NodeBody::VoxelLight` for `0x3D0C` nodes (skipped word, sRGB colour, skipped value, intensity, radius in cm, skipped word); a short body is invalid, not a panic. `chunk::tag::VOXEL_LIGHT` replaces `MESH_VC`
- [x] 1.2 `Material` decodes the fall-off amount and exponent (`0x64`, `0x68`)
- [x] 1.3 `Item::model_flags` (the word after the model's name) and `Item::light_share`, (1 − c/255)² with c bits 16–23 of the placement flags when bit 0 is set
- [x] 1.4 Unit tests on synthetic bytes: a voxel-light node, a truncated one, the model-section word and the condition share

## 2. World

- [x] 2.1 Model nodes keep their voxel light; spawned nodes get `VoxelLight` (lin(rgb)·√intensity·3) in level models, props and placed objects
- [x] 2.2 Placed items with the condition flag scale the voxel lights under their root, and the emissive colour of the meshes under it
- [x] 2.3 Placed objects' point lights count in `LevelInfo::lights`

## 3. GI

- [x] 3.1 Voxel lights reaching the block, with their voxel box, in their own list of up to 64 in the propagation's uniform buffer
- [x] 3.2 `gi.wesl` adds C·0.1·atn·0.25·sdw to the voxels each box holds, with the original's falloff and occlusion test, in every step and the warm-up
- [x] 3.3 Unit tests: the salve's colour and per-step injection, the reach (3.62 voxels open, about 1.2 blocked), the box sums 22.0 and 3.8, margins and bottom slices

## 4. Surfaces

- [x] 4.1 Unlit emissive alpha = opacity; emissive doubled for lit or specular records with an emissive layer
- [x] 4.2 Opacity fall-off for records above 0.01: alpha term, the opacity's floor, the lit emissive map's weight
- [x] 4.3 Additive surfaces draw (colour·alpha, 0), both clamped to 1; every surface's alpha clamped to [0, 1]
- [x] 4.4 Unit tests: the salve glow's emissive, fall-off and floor; a lit emissive map's doubling; the fall-off's alpha at the centre and outline

## 5. Verification

- [x] 5.1 Corpus (scratch probe outside the tree): 23 voxel lights, all decoded; the condition-flag items (salves, power crystals, Diviner's Deck, Repair Sphere, PedestalHand); the menu salve at share 1 and `exanima02`'s at 0.0044; `exa-tool corpus meshes` counts voxel lights per pack
- [x] 5.2 Menu at the original's camera (420 frames) against the original's capture: wall, shelf and cobwebs around the salve lit blue at about the capture's level
- [x] 5.3 `exanima01` at a light globe (wall lighting unchanged, the glass now an additive glow) and at its Healing Salve (blue light in the cart)
- [x] 5.4 Remove probes and dumps from the tracked tree

## 6. Documentation

- [x] 6.1 `docs/RENDERING.md`, `docs/formats/RFC.md`, `docs/formats/OTHER.md`, `docs/PLAN.md`
- [x] 6.2 `openspec validate --all --strict`
- [x] 6.3 fmt, clippy `--profile play --workspace --all-targets -D warnings`, tests of the touched crates
