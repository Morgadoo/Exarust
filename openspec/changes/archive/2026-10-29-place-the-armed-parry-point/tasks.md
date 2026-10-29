## 1. Research

- [x] 1.1 Map every routine of the class to its Exarust items (`docs/re/motiledyn-map.md`)
- [x] 1.2 Read the armed parry routine, its maths helpers and the stance by energy as instructions

## 2. Implementation

- [x] 2.1 `parry_blade_height`, `blade_point`, `parry_point`, `parry_distance`; `armed_grip` arithmetic and rounding (`exa_combat`)
- [x] 2.2 `engine_math::normalize_approx` (`exa_physics`)
- [x] 2.3 `stance::stance_by_energy` and the AI's guard choice

## 3. Verification

- [x] 3.1 Worked-number tests (synthetic data; EXANIMA_DIR is not read by these rules)
- [x] 3.2 Remote tests and clippy (-D warnings); cargo fmt

## 4. Docs

- [x] 4.1 Crate READMEs and `docs/INDEX.md`; docs/PLAN.md is updated by the round's rating, not this lane
