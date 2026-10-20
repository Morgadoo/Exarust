## 1. Research

- [x] 1.1 Re-read in REA/`exe.asm` the pathfinder's line walk, straight walk, obstacle test, cost sample, nearest-walkable search and its layer pick, the layer builder's byte pairs, the exploration steering's and combat walk's look-ahead, the footwork step start and the behaviour tick's spawn snap; constants from the PE
- [x] 1.2 Record the rules and the corrections (obstacle "within 40 cm" is lines over 40 cm; the snap once on the third tick; the straight walk's static floors; the off-floor search's leftover register) in `context/re/master-movement.md` §22 and the master plan's M5 note

## 2. Grid queries

- [x] 2.1 `exa_world::nav::query`: the line rasterised as the original's (major axis, minor rounded from the exact start, ties to even), the line walk (40 cm, 100 cm, the open underside, the highest floor kept) and its clamp (the last free cell's centre projected on the line, the start when the first cell blocks)
- [x] 2.2 The straight walk (the start's layer by height, 50 cm, 150 cm, following the floors), the obstacle test (over 40 cm, 70 cm, 150 cm, penalties over 60 before the last two cells), the cost at a point (×1/255), the layer a walker stands on
- [x] 2.3 The nearest walkable point (under 80 kept on its floor; the best-first flood by the planner's steps, 220 cells, the cheapest met; the off-floor search first) and `Nav`, the fallback without a grid

## 3. The player

- [x] 3.1 `steering`: the look-ahead's reach (75/45 + 6·v², −70), the line walks recorded for the overlay (look-ahead, soft stop, step), the spawn snap on the third tick unless down, footwork step targets (20 cm)
- [x] 3.2 `player`: the grid's line walk in the behaviour (the hip ray removed), the feet's simulated position (`CharacterBody::feet`), the spawn snap across the ground once the grid exists, the interim footwork walking only toward a step's target beyond 20 cm

## 4. Debug helpers

- [x] 4.1 `look_overlay`: the player's last line walk drawn with the grid overlay (`--debug-nav`, F6)
- [x] 4.2 `--trace` columns `goal_x`, `goal_z`

## 5. Tests

- [x] 5.1 Synthetic grids: a free line; a wall clamping at 135 cm and a slanted line's projection; the first cell; 40 cm up and a metre under; the highest floor kept against the straight walk following (105 cm, 50/60 cm, the pit); upper slabs (150 cm, 70 cm, open, a deck over a pit); cells without a floor; obstacles (40/120 cm lines, 70/80 cm, penalty 61/60, the last two cells); the cost (132/255, the upper layer); the layer pick; the nearest walkable point (kept on its floor, the diagonal neighbour at 60, off the floors at the base, 220 cells, the planner's 50/40 cm steps); the fallback
- [x] 5.2 Steering: the reach (92.137, 62.137, 45, −70), the recorded look-ahead and soft stop, the snap on the third tick only and not when down, step targets (20/25 cm, open); player: the footwork stops where its target is clamped within 20 cm; the trace's goal columns
- [x] 5.3 Corpus (skipped without `EXANIMA_DIR`): arena, arenao1 and exanima01 starts walkable and kept (exanima01's on its floor 10 cm up); line walks to the walls (arena ±4.95, 10.95, −4.95 m; arenao1 7.65, −7.95, 14.25, −13.65 m; exanima01 88.95, 73.65, 47.25, 31.05 m; its start's facing 5.947 m)

## 6. Verification

- [x] 6.1 Light runs through `gpu_run.sh --gpu 4000` in the arena: walking into the −Z wall (free walk ends with the goal clamped at −4.95 m, `0x50` at 48.7 cm, the capsule meets the wall at 1.1 cm a tick and stands braked 26.3 cm short with W held), jogging into it (`0x44` until the goal is 97 cm away, then `0x50`; contact at 2.3 cm a tick, no fall), the spawn snap logged (0.000 m), the overlay with the grid; DEBUGGING.md's movement script as before (walk 1.687, soft stop 2.25 cm short, `0x44` 4.337, `0x38` → `0xc`, backpedal 1.476); exanima01 not run (the RTX 4000 had 1.0–1.2 GB free; it ran out of memory there with 2.1 GB before)
- [x] 6.2 Remove probes and scratch files from the tracked tree

## 7. Documentation

- [x] 7.1 `docs/NAVIGATION.md` (Queries), `docs/PLAYER.md` (Movement, Combat mode), `docs/DEBUGGING.md` (overlay, trace columns), `docs/formats/RSQ.md` (trace columns), `docs/PLAN.md`, crate READMEs (`exa_world`, `exa_game`, `exa_physics`)
- [x] 7.2 `openspec validate --all --strict`
- [x] 7.3 `make check` equivalent on the `play` profile: `cargo fmt`, `cargo clippy --profile play --workspace --all-targets -D warnings`, `cargo test --profile play --workspace` with `EXANIMA_DIR`
