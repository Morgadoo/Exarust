## 1. Read the original

- [x] 1.1 The factory job builder's region list and region flags, the render callback's scratch, alpha, normal and specular passes
- [x] 1.2 `shademapa`, `gemstone_a`, `steel_n` in `textures.flb`; the `_procmatr` template
- [x] 1.3 The apparel layer-list builder: seed offsets, the wear table, the set fallback

## 2. Port

- [x] 2.1 The shade scratch and the regions' shade tiles (`atlas`, `paint::ShadeTile`)
- [x] 2.2 Overlay parts' regions (`weapon::paint_regions`)
- [x] 2.3 The alpha job and the refracting material
- [x] 2.4 The apparel seed offsets and the wear table by the stored byte

## 3. Tests and checks

- [x] 3.1 Unit tests (scratch blend, tile reads, steel turn, gem alpha, wear) and installation checks skipped without EXANIMA_DIR (overlay regions, gem weapons, the braies' tiling)
- [x] 3.2 `exa-tool paint-bench` before and after; `exa-tool corpus factories`
- [x] 3.3 fmt, clippy -D warnings, the workspace tests, openspec validate, the play build
- [ ] 3.4 Picture checks (owed; the GPU is on hold): a sword with a fuller, a gem-set weapon, a mirrored steel cap
