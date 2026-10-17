## 1. Holder and reach

- [x] 1.1 `CharacterStats::holder` (size, scale, encumbrance; weightless 0, no half-sword)
- [x] 1.2 `weapon_record::step_reach` in double precision (M41's arithmetic); `CharacterStats::step_reach` calls it and returns metres
- [x] 1.3 `exa_character`: `held_record` and `fist_records` take the holder; `equip_outfits` passes the fighter's (`Holder::AVERAGE` without a fighter)

## 2. Tests

- [x] 2.1 `weapon_record::step_reach_of_a_character`: 59.84 cm at feral 0.84 (double precision)
- [x] 2.2 `character_stats::weapons_are_held_by_the_character`: a new human holds as the average holder; a tall human at encumbrance 0.5 holds the 90 cm sword with reach 120.6 cm, recoil 0.67, shaping 0.275, blend 8
- [x] 2.3 `exa_character::the_fists_strike_across_the_hands`: the tall armoured holder's fists, arm stiffness 0.476, recoil 0.35
- [x] 2.4 Corpus tests unchanged (`shipped_weapons_are_held_with_their_records` with the average holder; the step reach corpus numbers of `character_stats`)

## 3. Documentation

- [x] 3.1 `docs/formats/OTHER.md` (Weapon record, Encumbrance), `docs/formats/RFC.md` (stats derived at spawn), `docs/formats/RSQ.md` (swing), `docs/PLAN.md`, `crates/exa_character/README.md`
- [x] 3.2 `openspec validate --all --strict`
- [x] 3.3 `make check` equivalent on the `play` profile: `cargo fmt`, `cargo clippy --profile play --workspace --all-targets -D warnings`, `cargo test --profile play --workspace` with `EXANIMA_DIR`
