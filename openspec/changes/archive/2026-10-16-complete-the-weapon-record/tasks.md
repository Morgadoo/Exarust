## 1. Research

- [x] 1.1 Confirm the plan's rules in REA and the disassembly: the record builder, the holding pass, the length and sweet-spot walks, the mass spread and its table, the equip refresh, the unarmed set-up and its three built-in sections, the hit-sound table, the shield record and its attach factors, the step reach; append the corrections to `context/re/combat-damage.md` (git-ignored)
- [x] 1.2 Corpus statistics with a scratch reader: the 208 weapon sections, the 22 shield sections, the races' weapon fields and clawed-hands words, the weapon models' node chains and collision bodies

## 2. Formats

- [x] 2.1 `rfc::Node`: the box's half extents and whether a collision body chunk follows the mesh body (`rigid_body`), GI points still read
- [x] 2.2 `objdb::Item::shield`: the shield section's record
- [x] 2.3 `weapon_record`: `Section` and the built-in fists, claws and heavy claws; `Lengths` (armed chain, one part, hand) and `chain` over a node list; `Holder`; `WeaponRecord::build` and `hold`; `RaceWeapons` (human constant, read from a race record); `unarmed`; `equip` (reach, race scaling, thrust preference, unarmed reach and hand mass, ghoul and Fael rules); `thrust_preference`; `step_reach`; `total_mass` and `spread_mass`; `ShieldRecord` and `ShieldFactors`

## 3. Game

- [x] 3.1 `exa_combat::damage` re-exports the record and its flags; the damage routine is unchanged
- [x] 3.2 `exa_character`: a wielded weapon's record built by `held_record` (a plain model's chain, a factory weapon's assembled top, the human holder and race); the blade's length is the record's
- [x] 3.3 `exa_ai`: the duellist's reach is its weapon's record reach, else the human's unarmed reach

## 4. Tests

- [x] 4.1 Unit tests pinned to the notes: the one-handed sword (impact 0.42, slash 0.397, thrust 0.325, W3 0.7, handling 0.7125, recoil 0.42, shaping 0.2, blend 7, arm stiffness 0.574, swing speed 1.125, turn 0.63, volume 1.2, pitch 1.022, sweet spot 57.6, reach 117.6, thrust preference 5); encumbrance 0.5 (0.67, 0.275, 8); the half-sword hold (0.75, 0.56) and the weightless item; two-handers, greatswords and weapons over 124 cm; the fists (17.66 cm, impact 0.43, 1.44, 0.1, 0, 0.5, turn 0.2, second hand 0.80032, reach byte 80) and claws (1.2, turn 0.6); the flipped side; chains (flail 84.4, torch 58.3, a rigid node past a plain one); unarmed sections by race (the Harvester's right fist and left claw); equipping (blend 3.5 → 4, no compounding, ghouls 1, Fael 0.315, unarmed reach 50 / 150 and hand mass 2 / 4); thrust preference 9, 1, 10; step reach; masses 0.28, 0.704, 0.85 and the spread; shield factors 0.875, 0.71, 0.473; race fields; the rfc box and collision body; the shield section
- [x] 4.2 Corpus (`exa_formats`, skipped without `EXANIMA_DIR`): the 208 weapon items (103 plain, 98 factory, 7 without a shipped model) with every holding-pass field in range and a mass; every plain model's box top equal to its highest vertex within 0.05 cm; Torch A01, Flail A, Ogre Flail, Sword GSF 01, Falchion 2H B, Ogre SpkClub pinned; the fists and claws of all 18 races from their body models' hands (human 17.66 cm, reach 50; ogre turn 0.5; Sir 150 and heavy; ghoul claws; Harvester; golem blend 0.5); the 22 shields (RoundShield R01)
- [x] 4.3 Corpus (`exa_character`): the 103 plain weapon items held through `held_record` without assembling a model; three factory weapons at their assembled top

## 5. Verification

- [x] 5.1 No in-game run: the change adds data and keeps a single-part weapon's length and reach as before (the box top is the highest vertex); held weapons, the damage routine and the duellist's stance distance are covered by the tests above
- [x] 5.2 Remove probes and scratch files from the tracked tree (the scratch readers stay in the session's scratch directory)

## 6. Documentation

- [x] 6.1 `docs/formats/OTHER.md` (object database sections, Weapon record), `docs/formats/RFC.md` (node box, collision body), `docs/PLAN.md` (Phase 4), `crates/exa_formats/README.md`, `crates/exa_combat/README.md`, `crates/exa_character/README.md`
- [x] 6.2 `openspec validate --all --strict`
- [x] 6.3 `make check` equivalent on the `play` profile: `cargo fmt`, `cargo clippy --profile play --workspace --all-targets -D warnings`, `cargo test --profile play --workspace` with `EXANIMA_DIR`
