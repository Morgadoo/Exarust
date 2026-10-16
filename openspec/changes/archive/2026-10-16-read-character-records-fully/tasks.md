## 1. Parser

- [x] 1.1 `exa_formats::characters`: `CharacterRecord` reads the whole record, bounded by its size: version, physique, height, age, voice pitch (face styles read by every version, as the original does), the worn list, then the feral level, `Pools` (yellow, red, focus), combat skill, undead voice, traits (byte − 100), experience, the six `Skill`s (points, technique bytes), the inventory (`Carried`), roles (`Role`), relations (counted), saved state (`SavedState`) and the powers sub-chunk (`Powers`); `Default`
- [x] 1.2 Legacy versions 5 and 6: roles as {id, word} added in turn, newest first, with the three first-role replacements (13 → 8 + 33, 27 → 26 + 33, 22 → 9 + 33); the version 5 and 6 relation shapes skipped; powers as 16-byte entries
- [x] 1.3 Strict counts (negative, more than six techniques, role blob longer than its entries, power blob shorter than its size word, a sub-chunk past the end, stray bytes); skill slots past the sixth skipped; checked sizes
- [x] 1.4 `Skill::learnt` (one per 5000 points, at most six), `Perk`, `Perks`, `CharacterRecord::perks`, `manoeuvring_tier` (highest of Armour 1–3), `deflection` (Armour 4), `voice_index`, `heal_pool` and `potion_expiry` (own entry), `Pools::shares`, `SKILL_NAMES`, `TECHNIQUE_NAMES`, `technique_name`
- [x] 1.5 `LevelCharacters::read_database` (every entry with its parse); `LevelCharacters::parse` uses it
- [x] 1.6 `exa_world`'s test helper builds its record with `..Default::default()` (one edit)

## 2. Tool

- [x] 2.1 `exa-tool characters <level|actors>`: records with looks, feral level, pools, combat skill, experience, traits, voice, skills and learnt techniques, perks, tier, Deflection, roles, counts, saved state, powers, and each placement

## 3. Tests

- [x] 3.1 Synthetic records (`characters_test.rs`): `whole_version_7_record` (every field, pools 0x8000 → 0.5000076, traits 100/50/200/0 → 0/−50/100/−100, roles with flags and variables, one relation, heal pool 0.25 and potion 1800, an unread sub-chunk, powers with a blob, an actor's database after the record), `learnt_techniques_give_perks` (15000 points learn three: Remise, Riposte, Feint but not Fend; the sixteen perks; 4999 points none; Dual Wield and Draw none), `manoeuvring_tier_is_the_highest_learnt` (I+II → 2; all and Deflection → 3; 65535 points clamp to six), `skill_slots_past_the_six_are_skipped`, `legacy_version_6_record`, `legacy_first_roles_are_replaced`, `legacy_version_5_relations`, `saved_entries_read_their_own_values`, `voice_numbers` (vi03 → 2, vm09 → 8, vi00 → 0), `malformed_records_are_errors` (every truncation, version 8, bad tag, seven techniques, negative count, role blob, power blob, stray bytes, overrunning sub-chunk), `placements_and_their_database`
- [x] 3.2 Corpus (skipped without `EXANIMA_DIR`) `shipped_level_records`: 13 sections, 478/20/4 records of versions 7/6/5, 26 deleted entries and 2 stubs; 477 placements all version 7, 451 living all with full pools, 97 living humans feral 0.40 ×1, 0.42 ×1, 0.50 ×27, 0.84 ×21, 1.00 ×47 and no other race feral; perk counts (Remise 358 … Volition 1); 37 learn Dual Wield; 5 with powers; no relations or saved state
- [x] 3.3 Corpus `shipped_actors`: 21 records, 14 version 6; opponent16 perks, tier 3, Deflection; opponent13's shield perks; empty pools for every opponent and the proctor, full for the rest; the demon (race 11, skill 100); Derrin's roles
- [x] 3.4 `exa-tool characters exanima01` (42 records, 2 entries not records, 39 placed), `characters actors` (21) and `characters exanima08` (63, the thaumaturges' powers) against the installation

## 4. Verification

- [x] 4.1 REA and `exe.asm`: both record readers, the inventory and relation readers, the technique routine, the armour builder's tier and Deflection (0.3 per tier), the save hook and load (1/65535, last-entry quirk), the name tables; findings and corrections appended to `context/re/master-body.md` (§17)
- [x] 4.2 No in-game run: nothing consumes the values, so no behaviour changes (unit and corpus tests only)

## 5. Documentation

- [x] 5.1 `docs/formats/RFC.md` (character records: the whole layout, versions, perks, evidence), `docs/PLAN.md` (Phase 3), `crates/exa_formats/README.md`, `crates/exa_tool/README.md`
- [x] 5.2 `openspec validate --all --strict`
- [x] 5.3 `make check` equivalent on the `play` profile: `cargo fmt`, `cargo clippy --profile play --workspace --all-targets -D warnings`, `cargo test --profile play --workspace` with `EXANIMA_DIR`
