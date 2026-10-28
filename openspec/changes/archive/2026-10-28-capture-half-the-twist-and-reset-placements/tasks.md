## 1. Research

- [x] 1.1 Read the start capture, the may-act gate, the plant-request lapse, the struck-item voice and the placement routine in the disassembly

## 2. Implementation

- [x] 2.1 `half_lean`, `Locomotor::twist_before`, `limp::Lean` in the sampler's clock
- [x] 2.2 The duellist's step gate reads the trailing-foot request
- [x] 2.3 `struck_item_sound`
- [x] 2.4 `Locomotor::placed`, `Ragdoll::clear_leg_corrections`

## 3. Verification

- [x] 3.1 Worked-number tests for each rule (synthetic data; no real-data check applies, EXANIMA_DIR is not read by these rules)
- [x] 3.2 Remote tests and clippy (-D warnings) for exa_physics, exa_combat, exa_game, exa_ai, exa_anim; cargo fmt

## 4. Docs

- [x] 4.1 Crate READMEs; docs/PLAN.md is updated by the round's rating, not this lane
