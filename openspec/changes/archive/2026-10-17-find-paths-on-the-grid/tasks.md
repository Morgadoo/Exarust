## 1. Research

- [x] 1.1 Read in REA/`exe.asm` the A\* (window, layer pick, expansion, cost, heuristic, waiting list, node limit, fallback, pull-back, corner walk, string pulling) and its line test, the AI planner and its slot, the slot's hand-over in the simulation step, the footprint stamp, removal and callers, the AI's default goal, the dirty-rectangle slots and the pathfinder's tick; tables and constants from the PE
- [x] 1.2 Record the rules and corrections (the slot passes per simulation step; the string pulling's own line test and budget; the footprint's ring rule; the dead pull-back loop; the window wrap) in `context/re/master-movement.md` §23 and the master plan's M6 note

## 2. Planner

- [x] 2.1 `exa_world::nav::path`: `PathGoal` (AI defaults 3, 1, 10), the window and its limits, the start's layer, the expansion by the shared step rule (40 cm, 2 m, 180/(size + 1)), the cost (10·Δh², 5/7, 2·clearance, shifted penalty ≤ 600), the heuristic, the waiting list's order, cheaper re-met cells, 4095 cells
- [x] 2.2 The end: success within the stop distance, the nearest cell, the goal's test or 100 cm, the pull-back before rough cells; corners; string pulling with the line test (+10, penalties shifted once more); points at cell centres; `Nav::find_path` without a grid
- [x] 2.3 `PlanSlot` and the `NavPlanSlot` resource, handed over each fixed tick

## 3. Footprints and movers

- [x] 3.1 `exa_world::nav::footprint`: stamp (5 × 5, layer by height, 255/128/+64, quarter added, wrapping), removal, the edge; `NavFootprints`
- [x] 3.2 `exa_game::footprints`: NPCs every tick, the player while not walking, lifted for characters that leave; cleared with a new level's grid
- [x] 3.3 `exa_world::nav::dirty`: built floors kept beside the movers', the straight walk on built floors, `rect_around`, `queue_moved` (ring of 16), `mark_dirty` (merge), `refresh` (restore, raise rule, cost passes where changed); the game's tick refresh (no mover yet)

## 4. Debug helpers

- [x] 4.1 `exa-tool nav --from --to [--size --shift --stop]`: plan, print, draw on the picture
- [x] 4.2 `exarust --debug-path x,y,z` (`exa_game::path_probe`): plans through the slot with the player's footprint lifted, re-plans after a metre, logs and draws; the nav overlay's penalty squares

## 5. Tests

- [x] 5.1 Synthetic grids: straight ways (50, 35); 40/50 cm steps (370), 2/2.1 m drops (4050); a slab 90/100 cm over; clearance 45/46 (class 3), 180/181 (class 0); penalties 600/608, 510; a rough step (100); stop distance 50 (4 short, 30); 4095 cells and the nearest (30 cm near enough, 150 cm not); the goal's own test; the pull-back at 17 against 16; the L corridor (57, 4 points) and a greedy bend pulled straight (56, 2 points); the line test's budget (50 against 40/39 + 10, a penalty of 5 as 20, no floor, 50 cm up); a bridge by height; the window limits (253/254 apart, cell 22/23); footprints on a corridor (318 more; shift 4 refuses); no grid; the slot
- [x] 5.2 Footprints (63/32/16, wrapping, the upper layer, the edge) and movers (the rectangle 16–24 and the edge, merge and ring, a refresh raising a floor 40 cm and restoring it, the raise rule); the game's footprints following their characters; the debug probe planning through the slot with the player's footprint lifted (cost 20, not 116)
- [x] 5.3 Corpus (skipped without `EXANIMA_DIR`): the arena's way south (87 cells, cost 135, two points); exanima01 round the crossing (810 cells, cost 659, 11 points, 22.75 m) and from the start toward it (2039 cells, ends at (73.65, 0, 40.35))

## 6. Verification

- [x] 6.1 Light run through `gpu_run.sh --gpu 4000` (or say why it could not run): `--still-opponent --debug-nav --debug-path 0,0,-3 play arena`
- [x] 6.2 Remove probes and scratch files from the tracked tree

## 7. Documentation

- [x] 7.1 `docs/NAVIGATION.md` (Planning, Footprints, One plan a tick, Moving things, Exarust, not reproduced), `docs/DEBUGGING.md`, `docs/PLAYER.md`, `docs/PLAN.md`, crate READMEs (`exa_world`, `exa_game`, `exa_tool`)
- [x] 7.2 `openspec validate --all --strict`
- [x] 7.3 `make check` equivalent on the `play` profile: `cargo fmt`, `cargo clippy --profile play --workspace --all-targets -D warnings`, `cargo test --profile play --workspace` with `EXANIMA_DIR`
