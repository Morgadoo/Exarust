## 1. Parser

- [x] 1.1 `exa_formats::races`: `RaceBody` gains the clip-rate base (`0x5C`), size (`0x64`), scale (`0x68`) and mass divisor (`0x70`); `SCALE_CAP` (1.5); `RaceBody::FALLBACK` with 1.0 for each
- [x] 1.2 `Gait::brake_distance` from the loader's built-in column (`BRAKE_DISTANCES` 0.5, 2, 2.5, 0.5 m), the fallback's rows included; `RaceBody::sprint_gait(feral)` (sprint row at 0, moved toward the jog row by the feral level)
- [x] 1.3 `RaceStats` (`Race::stats`): second flag word (`0x1C`), kind word (`0x58`), unarmed reach (`0x84`, m), swing turn (`0x88`), AI attack bonus (`0x8C`), recovery (`0x90`), perception (`0x110`), held offset (`0x12C`, signed bytes × 0.1 cm, m), footsteps (`0x144`, ≤ 15 characters), voice volume (`0x1B4`), attack voices (`0x1C4`); `RaceStats::FALLBACK` from the built-in record; `stats_of_character`
- [x] 1.4 Flag constants `MAY_RISE`, `REGENERATES_RED`, `HOVERS` (first word), `REGENERATES_RED_TOO`, `QUADRUPED` (second), `CLAWED` (kind); `Race::regenerates_red`, `Race::clawed`
- [x] 1.5 `exa_combat`'s synthetic race fills `stats` (one line)

## 2. Tool

- [x] 2.1 `exa-tool races`: flag words, clawed and red regeneration, clip-rate base, size, scale, mass divisor, the stats, and each gait row's brake distance

## 3. Tests

- [x] 3.1 Synthetic records: `body_parameters` (every new body field from its offset, column 2 replaced by the brake distances), `stats` (each stat from its offset, signed held-offset bytes 90/−10/5 → 0.09/−0.01/0.005 m, the footstep set cut at 15 characters, the flag helpers), `feral_characters_sprint_toward_the_jog` (feral 0 and below keep the sprint row; 0.5 → thrust 411.1, clip rate 15.05, brake 2.25 m; 1 → the jog row), `unknown_races_fall_back` (body and stats fallbacks: brake distances, 1 m reach, voice volume 0, no footsteps)
- [x] 3.2 Corpus (skipped without `EXANIMA_DIR`) `shipped_races`: every one of the 18 records' clip-rate base, size, scale, mass divisor, flag words, kind, reach, swing turn, recovery, perception, footsteps, voice volume, attack voices, attack bonus and held offset against the notes' table; who regenerates red, is clawed, hovers, may rise and is a quadruped
- [x] 3.3 `exa-tool races` against the installation (18 races, 10 lines each; Ghoul `0x3` / `0x3`, kind `0x100`, clawed, clip rate 1.5, reach 1 m, `StpSft`; the demon regenerates red, held offset 0.090 m; every row's brake 0.5 / 2 / 2.5 / 0.5 m)

## 4. Verification

- [x] 4.1 REA and `exe.asm`: every read of a race field through the character's race pointer and the builder's entry, the constants (1.5, 0.05, 83.0/83, the built-in column, the built-in record, 0.1 cm, the perception factors 0.9/0.8 and range 1200 cm); findings and corrections appended to `context/re/master-body.md` (§16)
- [x] 4.2 No in-game run: nothing consumes the new values, so no behaviour changes (unit and corpus tests only)

## 5. Documentation

- [x] 5.1 `docs/formats/OTHER.md` (Races: every decoded field, its meaning, units and evidence; the brake column; the fallback), `docs/PLAN.md` (Phase 3), `crates/exa_formats/README.md`, `crates/exa_tool/README.md`
- [x] 5.2 `openspec validate --all --strict`
- [x] 5.3 `make check` equivalent on the `play` profile: `cargo fmt`, `cargo clippy --profile play --workspace --all-targets -D warnings`, `cargo test --profile play --workspace` with `EXANIMA_DIR` (339 passed, 3 ignored, 0 failed; `exa_formats` races 7 of 7, the corpus test on the installed data)
