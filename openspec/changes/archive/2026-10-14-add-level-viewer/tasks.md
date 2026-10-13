# Tasks

## 1. Decoding

- [x] 1.1 Add `exa_formats::level` with tile-grid and prop types; parse them in `Scene`
- [x] 1.2 Unit tests on synthetic grid and prop chunks
- [x] 1.3 `exa-tool corpus levels` and a run over every level in the installation (skipped without `EXANIMA_DIR`)

## 2. Bevy conversion

- [x] 2.1 Mesh, material and texture conversion with a per-name cache in `exa_assets`
- [x] 2.2 Model loading by `(pack, entry)` with node hierarchy, shared between placements

## 3. Viewer

- [x] 3.1 `exa_world` level spawning: level nodes plus props from the object library, missing data logged once
- [x] 3.2 Free-fly camera and stand-in lighting
- [x] 3.3 `exarust model <pack> <entry>` and `exarust level <name>`
- [x] 3.4 Screenshot check of a model, an arena and a story level

## 4. Documentation

- [x] 4.1 Update `docs/formats/RFC.md` level sections and `docs/PLAN.md` Phase 2 status
- [x] 4.2 `make check`
