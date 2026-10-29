## 1. Research

- [x] 1.1 Read the captures, the muscles-off sampler's lead-in and feet block, the swing voice, the held-voice fade, the walking rustle and the foley set-up in the disassembly

## 2. Implementation

- [x] 2.1 Captured planted feet and the muscles-off lead-in (`exa_anim`)
- [x] 2.2 `limp_foot`, `limp_anchor_height`, `limp_step`, `walk_rustle` (`exa_physics`) and `sample_limp` (`exa_game`)
- [x] 2.3 `exa_combat::swing_voice`

## 3. Verification

- [x] 3.1 Worked-number tests for each rule (synthetic data; no real-data check applies, EXANIMA_DIR is not read by these rules)
- [x] 3.2 Remote tests and clippy (-D warnings); cargo fmt

## 4. Docs

- [x] 4.1 Crate READMEs; docs/PLAN.md is updated by the round's rating, not this lane
