## 1. Grid top

- [x] 1.1 `exa_formats::environment`: `Environment::grid_top` (record `0x90`, `0x98` when it is 0, 3.4 m without the record); tests `reads_ambient_extent_and_sun`, `a_zero_grid_top_takes_the_highest_point`

## 2. The grid

- [x] 2.1 `exa_world::nav`: constants (30 cm, 10 cm, 20 cells a room, 320 default, 256 voxels, 25.5 m, headroom 12, 3 spans, sky above 240, underside limit 235, weights 256/181, 120/60/240, cap 120, passable 180, floor lift 0.5), `Columns` (stamp, ground fill, rounding in snapped centimetres), `column_spans` (the original's byte-at-a-time scan with its cursor), `step_measure`, `met_layer`, `NavGrid` (layers, roughness, blur, clearance, zeroed penalty; cell queries, centre, cell of a point, floor height, passable, stats)
- [x] 2.2 `nav::source`: `NavSource` (size, base, meshes' points with transforms, floor meshes, ground tiles) and `build`; `grid_size`, `grid_base`; `GroundTile` (bilinear, holes read a byte at a time); `floor_points` (10 cm columns, 20 cm cubes on the world lattice)
- [x] 2.3 Tests pinned to the notes: `spans_need_more_than_120_cm_above` (undersides 216 and 217 under a floor at 221, a ceiling from 235 joining a floor at 221 and not one at 219, a floor at 199 read as 198), `spans_drop_high_undersides_and_cap_at_three`, `points_mark_the_voxel_and_cell_they_round_to` (voxel 221 at 0 m, ties to even), `ground_fills_columns_to_its_height`, `steps_walls_and_drops` (4, 8, 14, 15; 60 and 42), `walkers_meet_the_layer_within_150_cm`, `clearance_grows_toward_an_edge` (132, 60, 18, 0), `a_step_is_rough_on_both_sides` (16), `a_flat_6_m_floor_gets_900_points_10_cm_under_it`, `ground_tiles_sample_bilinearly_and_read_holes_a_byte_at_a_time`, `grids_cover_the_tile_grid` (640 × 800), `a_floor_and_a_wall_from_their_points` (wall top open sky; beside it 32, 68, 68)

## 3. In game

- [x] 3.1 `nav::view`: `NavPlugin` (in `WorldPlugin`), `PendingNav` inserted by `LevelLoad::finish` (root, size, base), collection after transform propagation, build on the async compute pool, `NavGrid` resource and `NavState`; `Ground` on each terrain tile (displaced heights and holes)
- [x] 3.2 `nav::headless::level_source`: the level spawned into a bare world with world transforms from the parents, the same inputs collected

## 4. Debug helpers

- [x] 4.1 Overlay: `exarust --debug-nav`, F6; the floor cells within 8 m of the floor the camera looks at, coloured by layer and clearance; test `the_overlay_finds_the_floor_the_camera_looks_at`
- [x] 4.2 `exa-tool nav <level> [--png file]`: counts and both layers drawn

## 5. Verification

- [x] 5.1 REA and `exe.asm`: the build, stamp (rounding, bounds, filter, the `+Floor` extra height 0.5), terrain fill and sampler, span constants and the scan's byte-at-a-time walk (an emulation of its instructions against `column_spans` over 222 000 columns), layer split, region flood (a count only), movable-prop raster (floor bytes), cost passes (weights read from the exe, constants, rectangles), grid top from the record and the loader's fill-in; notes in `context/re/master-movement-19-nav-grid.md` (for `master-movement.md` §19)
- [x] 5.2 Corpus with `EXANIMA_DIR`: environment records and tile grids of every level (probe); `shipped_levels` pins arena (640 × 640, 1935 floors, 531 sky, reach 1856), arenao1 (25144 + 4847 floors, reach 5050), exanima01 (640 × 800, 385525 + 854 floors, 121865 sky, reach 338283, the stair room's foot at 211 reached); `exa-tool nav` pictures of the three
- [x] 5.3 In game through `gpu_run.sh --gpu 4000` (`--debug-nav --no-gi --no-supersample --size 960x540 play arena`), in the verification run: the arena's floor is drawn green around the player, cut by the walls, yellow at their feet, and the log gives the corpus test's counts (1935 floors, 531 open sky). The same light run of exanima01 built its grid in game with the corpus test's counts but ran out of GPU memory before drawing (the RTX 4000 had 2.9 GB free)
- [x] 5.4 Remove the probes from the tracked tree

## 6. Documentation

- [x] 6.1 `docs/NAVIGATION.md` (new), `docs/formats/RFC.md` (Environment `0x90`), `docs/DEBUGGING.md` (`--debug-nav`, F6, `exa-tool nav`), `docs/PLAN.md` (Phase 6), `docs/INDEX.md`, READMEs of `exa_world`, `exa_tool`, `exarust`
- [x] 6.2 `openspec validate --all --strict`
- [x] 6.3 `make check` equivalent on the `play` profile: `cargo fmt`, `cargo clippy --profile play --workspace --all-targets -D warnings`, `cargo test --profile play --workspace` with `EXANIMA_DIR`
