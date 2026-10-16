## 1. Skin binding

- [x] 1.1 `exa_assets::SkinnedPart` keeps the mesh handle; `exa_world` and `exa_character` spawn skinned parts without `Mesh3d`
- [x] 1.2 `exa_anim::bind_skins` inserts `Mesh3d` with `SkinnedMesh`
- [x] 1.3 `exa_game::limp` part boxes and `exa_character`'s covered-part hiding read skinned parts before binding

## 2. Verification

- [x] 2.1 `character` through `gpu_run.sh`, 5 runs: no validation error, body and braies drawn skinned in the walk
- [x] 2.2 `play exanima01 --at 68.8,0.3,-48.3`, 2 runs (4 of 4 failed before): no validation error
- [x] 2.3 `play arena --opponent`: player and opponent drawn

## 3. Documentation

- [x] 3.1 `docs/formats/RSQ.md`
- [x] 3.2 `openspec validate --all --strict`
- [x] 3.3 `make check`
