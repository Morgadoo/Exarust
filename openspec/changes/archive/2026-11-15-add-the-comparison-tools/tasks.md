## 1. Outdoor seed

- [x] 1.1 `fix_seed`/`fixed_seed` in `environment`; `clock_seed` and the preset's fresh-seed fallback read it
- [x] 1.2 `--env-seed` (nonzero, decimal or hex) in the launcher; zero refused (exit 2)
- [x] 1.3 Test: a fixed seed repeats the outdoor draw; three `arenao4` runs with one seed in one grid: two pixel-identical, the third 202 scattered pixels apart (0.022 %, FPS counter left out), at the noise floor; another seed changes 93 % of the picture

## 2. New game

- [x] 2.1 `new_game::NewGamePlugin`: the story start and the session begin once the load has ended
- [x] 2.2 `--new-game story[:N]`; `arena` refused with exit 2 (the career start is the creator's commit)
- [x] 2.3 Test: the first step is the new slot, the flow is in story mode, the player is the created character; grid run `play exanima01 --new-game story --save-dir <scratch> --debug-exit 600` left `Exanima001.rsg` (exanima01 and exanima02 sections, rebuilds byte for byte) and `Exanima001.rcp` in the folder

## 3. Picture diff

- [x] 3.1 `exa-tool picdiff` with threshold, tiles, ignored rectangles, `--fit`, `--noise`, `--out`, `--json`
- [x] 3.2 Tests on synthetic pictures

## 4. Docs

- [x] 4.1 Crate READMEs and `docs/DEBUGGING.md` (`--env-seed`, `--new-game`, `picdiff`)
