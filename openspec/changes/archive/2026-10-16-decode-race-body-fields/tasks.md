## 1. Parser

- [x] 1.1 `exa_formats::races`: `RaceBody` (support-pin factor, balance bonus, stance factor, arm relaxation, damping ratio, four `Gait` rows of thrust, push scale, clip rate and boost speed) decoded into `Race::body`
- [x] 1.2 `RaceBody::FALLBACK` (every factor 1.0, zero gait) and `body_of_character`, which returns it for a race byte the database lacks

## 2. Tool

- [x] 2.1 `exa-tool races`: every race with its names, model, movesets, health, body parameters and gait rows

## 3. Tests

- [x] 3.1 Synthetic records: every field read from its offset, gait rows in order, column 2 ignored, the fallback for a missing race
- [x] 3.2 Corpus (skipped without `EXANIMA_DIR`): every `races.rdb` record parses; Human 1.25 / 1.0 / 0.5 / 1.0 / 1.0, gait thrusts 88.9 / 350 / 472.2 / 77.8 with push scale 550 in every row; Skeleton stance 0.75 and arm relaxation 0.5
- [x] 3.3 `exa-tool races` against the installation (18 races; Human pins 1.25, bonus 1, stance 0.5, arms 1, damping 1, walk thrust 88.889)

## 4. Verification

- [x] 4.1 In-game regression (no behaviour change expected), through `gpu_run.sh`: R3 (`--shove 60 play arena`: falls at tick 305, gets up with `Get Up 2` at 511, idle again at 657), R5 (`play exanima01`: 39 characters placed, 7 corpses collapse), R6 (`--limp character`: drops to a sprawl, still from tick 175)

## 5. Documentation

- [x] 5.1 `docs/formats/OTHER.md` (Races: the fields, their meaning and evidence, the fallback, the loader's per-tick scaling), `docs/formats/RSQ.md` (Physical body: where the race factors come from), `docs/PLAN.md` (Phase 3), `crates/exa_formats/README.md`, `crates/exa_tool/README.md`
- [x] 5.2 `openspec validate --all --strict`
- [x] 5.3 `make check` (fmt, clippy -D warnings, tests; `play` profile)
