## 1. The sum

- [x] 1.1 `exa_formats::objdb`: `Burden` from every item body (encumbrance `0x1A5`, covered bones from the coverage words, articulated set `0x220`, special kind `0x214`, powered: flag byte `0x218` bit 0 and charge `0x219` not 0xFF; bytes past a short body read 0), `worn_as`; the shield's bulk from main's shield record (`Item::shield`, its bytes now scaled in double)
- [x] 1.2 `exa_formats::encumbrance::encumbrance(worn, shield, skills)`: per item (golem suit +2 unpowered, gauntlets' weightless shield, + byte × 0.0025 in double), articulated pieces touching another (any set, the original's neighbour table) −0.025 each, −0.3 per Manoeuvring tier, floor 0.001, shield (2 − weightless)·bulk/4 (`ShieldRecord::encumbrance`) − 0.3 with Mobility when positive, cap 1; `Skills::of_record`, `of_record`, `held_shield` (the last of the first two held items that is a shield and no weapon)
- [x] 1.3 The consumers' factors: `run_cap` (0.3, 0.4 sprinting), `sprint_clock` (0.1), `turn_rate` (0.6), `standing_alignment`, `long_step` (1.5 − 0.3), `footwork_long_step` (1.6 − 0.3), `get_up_rate` (1.5 − 0.5, float)
- [x] 1.4 `characters::actor_items` (the database after an actor's record; none after the demon's)

## 2. Consumers

- [x] 2.1 `CharacterStats::encumbrance` (`with_encumbrance`) and `deflection`
- [x] 2.2 `exa_combat::damage::armour_layers(natural, worn, deflection)`: stored protection (no second condition scaling; padding in double, resistances × 1/140 in float), `coverage` with Deflection's bonus; `Fighter::of_stats` and dressing pass Deflection
- [x] 2.3 `exa_world::PlacedCharacter::encumbrance` from the record and the level's or the game's items; `viewer.rs`: placed characters with it, the player and the duellist from their outfits (logged at spawn)
- [x] 2.4 `exa_game::player`: the run blend rises at most to `run_cap(encumbrance)` (`run_blend`)
- [x] 2.5 `exa-tool characters` prints the encumbrance and the step reach at it (levels' and actors' own databases)

## 3. Tests

- [x] 3.1 Unit (`encumbrance_test.rs`): `bytes_add_up` (0.175), `articulated_pieces_touching` (0.3, 0.25, 0.3), `the_left_arm_reaches_the_right_shoulder` (0.05, 0.1), `manoeuvring_takes_off_a_third` (0.2, 0, 0), `a_shield_adds_its_bulk` (0.3, 0, 0, 0.2, 0.35, 0.3), `the_golem_suit_and_the_cap` (1, 0.1, 1, 1), `what_encumbrance_slows` (0.85, 0.8, 0.95, 0.7, 0.5, 1.35, 1.45, 1.25), `a_records_encumbrance` (0.4, 0.3, 0; the shield's hands)
- [x] 3.2 Unit: `objdb` (`burden_of_an_articulated_piece`, `powered_specials`, `shield_section`: bulk 60 is 0.6), `character_stats` (`encumbrance_after_the_init`), `damage` (`deflection_covers_more`, `stored_protection_is_not_scaled_again`), `exa_world` (placed encumbrance 0.025), `player` (`armour_weighs_the_run_down`: 1, 0.85, 0.835)
- [x] 3.3 Corpus (skipped without `EXANIMA_DIR`): `shipped_level_encumbrance` (451 living, 386 at 0, 31 distinct values, 27 shields, Elestan 0.77, Demian 0.67), `shipped_actor_encumbrance` (opponent03 0.065, 05 0.225, 07 0.475, 10 0.425, 13 0.07, 15 0.34, 16 0.06, proctor 0.195, shopkeeper 0.025; the demon, villager, Derrin, opponents 1 and 8 none), `shipped_outfits` (the duellist's 0.175, braies 0, tourney plate 0.625)
- [x] 3.4 `exa-tool characters actors` and `characters exanima01` against the installation (opponent07 0.475 and step reach 72.9 cm; 10 of exanima01's records encumbered)

## 4. Verification

- [x] 4.1 REA and `exe.asm`: the armour pass (constants from the image), its special handlers, the spawn order (shield attached first), the shield record, every reader of the encumbrance, the item instance rule; a corpus count of the level instances' stored bytes; findings and corrections in `context/re/master-body.md` §19
- [x] 4.2 In game (light runs on the RTX 4000, `--no-gi --no-supersample --size 960x540`): `play arena` running to a far cursor at 3.61 m/s unarmoured and 3.24 m/s in the tourney plate (encumbrance 0.625, run blend at most 0.8125, logged); `--opponent play arena`: the duellist spawns at 0.175 and duels as before (no blow lands in 1500 frames with this branch or its base)

## 5. Documentation

- [x] 5.1 `docs/formats/OTHER.md` (Armour, Encumbrance), `docs/formats/RFC.md` (stats derived at spawn), `docs/PLAYER.md` (the run cap), `docs/PLAN.md` (Phase 4), crate READMEs (`exa_formats`, `exa_combat`, `exa_world`, `exa_tool`)
- [x] 5.2 `openspec validate --all --strict`
- [x] 5.3 `make check` equivalent on the `play` profile: `cargo fmt`, `cargo clippy --profile play --workspace --all-targets -D warnings`, `cargo test --profile play --workspace` with `EXANIMA_DIR`

## 6. Merge onto main

- [x] 6.1 Rebased onto main (weapon record, object bodies and mechanisms): `objdb::Item` keeps main's `shield`, `door` and `operative` beside `burden`; `encumbrance` and `held_shield` take the shield record; `ShieldRecord::from_section` scales its bytes in double (15 → 0.15, 60 → 0.6); the dressing pass keeps main's fists and passes Deflection
