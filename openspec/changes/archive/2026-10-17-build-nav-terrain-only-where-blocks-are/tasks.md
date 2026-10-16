## 1. Ground blocks

- [x] 1.1 `exa_world::nav::source`: `TILE_BLOCKS` (the terrain's 8), `GroundTile::blocks` (bit `bx` of row `by`), `ALL_BLOCKS`, `with_blocks`, `has_block`
- [x] 1.2 `terrain::TerrainBuild`: a tile's `Ground` takes the tile's `TileBlocks` presence in a tile-grid level (every block without one)

## 2. Terrain fill

- [x] 2.1 `NavSource::build` fills terrain a block at a time: first cell half the grid plus the block corner over 30 cm, rounded; 20 × 20 cells sampled at vertex (i + ½)·1.5 + 30·block index; absent blocks skipped
- [x] 2.2 Test pinned to the notes: `ground_fills_only_the_blocks_that_exist` (every block 25 600 cells; blocks (0, 0) and (5, 2) 800 cells, cell (20, 20) at voxel 221, cell (125, 65) at 224 as the whole tile gives it, block (1, 0) empty)

## 3. Verification

- [x] 3.1 REA and `exe.asm`: the grid build's terrain loop over each tile's block list, the block builder, the terrain build (grid mode: presence builder; else 64 blocks a tile), the presence builder (flagged blocks built, unflagged removed, a flat tile at 0 for a flagged empty slot), the level load (terrain, then the load-end hook that builds the grid); notes appended to `context/re/master-movement.md` §19 and `context/re/black-fill.md`
- [x] 3.2 Corpus with `EXANIMA_DIR`: `shipped_levels_flag_no_empty_tile_slot` pins that all 17 levels with a terrain have a grid and flag no cell in an empty tile slot; `shipped_levels` pins arena (unchanged), arenao1 (14 677 + 3657 floors, 2867 + 270 sky, reach 5050, 34 blocks) and exanima01 (219 292 + 792 floors, 101 745 + 104 sky, reach 190 954, 682 blocks, the stair room's foot at 211 reached, 477 closed cells with 1120 floor cells over them, 186 721 and reach 338 283 with whole tiles); `exa-tool nav` counts and pictures of exanima01, arenao1, arenao3, arena7 and arena before and after
- [x] 3.3 In game through `gpu_run.sh --gpu 4000` (`--debug-nav --no-gi --no-supersample --size 960x540 --screenshot … play <level>`): arenao1 logs the corpus counts and draws the arena floor; exanima01 builds its grid with the corpus counts, then runs out of GPU memory before its screenshot (about 2 GB free on the RTX 4000)
- [x] 3.4 Remove the one-off probes from the tracked tree (the empty-slot check stays as a corpus test)

## 4. Documentation

- [x] 4.1 `docs/NAVIGATION.md` (terrain blocks, the numbers, the guess about the ground between rooms replaced), `docs/formats/TERRAIN.md` (the grid reads the built blocks; the empty-slot tile; 477 closed cells), `docs/DEBUGGING.md` (the `exa-tool nav exanima01` output), `docs/PLAN.md` (Phase 6), `crates/exa_world/README.md`
- [x] 4.2 `openspec validate --all --strict`
- [x] 4.3 `make check` equivalent on the `play` profile: `cargo fmt`, `cargo clippy --profile play --workspace --all-targets -D warnings`, `cargo test --profile play --workspace` with `EXANIMA_DIR`
