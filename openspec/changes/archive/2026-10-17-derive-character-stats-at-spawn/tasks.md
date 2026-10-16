## 1. Derivation

- [x] 1.1 `exa_formats::character_stats`: `CharacterStats::derive(record, race, Spawn)` with the builder's clamps (height −1…1, feral 0…1), scale (cap 1.5, then 1 − 0.05·h, in double as the original), size, standing pelvis 109.5 − 11.25·h cm, leg reach 107.2 − 11.1·h cm, gait clock 1/(1 − 0.1·h), most energy, support pins' factor, head gain and factor, walking arms, sprint gait, clip rate, perception, voice pitch (0.85 for the hulking ghoul) and volume, attack-voice step; `step_reach(encumbrance)`; `Default` (a new record of no race)
- [x] 1.2 Maximum health with the init's two race-byte blocks; yellow and red from the pools (× 1.5259022e-5); a living placement's unsaved red pool refilled with yellow
- [x] 1.3 `BodyRules` with the original's bits: by race byte, by the race's flag words, by the feral level, the armour pass's golem bit (cleared by a worn core)

## 2. Consumers

- [x] 2.1 `exa_physics::BodyStats` on `Physical` and the drive (`Ragdoll::with_stats`): energy recovers up to the most energy; the head's pose gain × head gain; the head's factor × head factor (`set_part_factors`); the walking arms in the walk and run cycles (after the dazed 0.2); limbs' factors 1.25 × scale
- [x] 2.2 `exa_combat::Fighter::of_stats` (health from the stats, natural armour and bleeding from the race; `Fighter::stats`) replacing `of_race` and `of_record`
- [x] 2.3 `exa_world::PlacedCharacter::record`; `exa_game` limp builds bodies with `Physical::stats`; `viewer.rs` derives every placed character's stats (story levels by `start::mode`), the player's and the duellist's from a new human record
- [x] 2.4 `exa-tool characters` prints the stats each record spawns with

## 3. Tests

- [x] 3.1 Unit (`character_stats_test.rs`): `height_scales_the_body` (h = 1: 0.95, 98.25 cm, 96.1 cm, ×1.111; h = −1: 1.05, 120.75 cm, 118.3 cm, ×0.909; h = 3 kept at 1), `the_scale_is_capped_before_the_height` (1.575, 1.89), `feral_weakens_the_undead` (f = 1: 0.9, 1.0, 0, 0.2, 0.05, 0.5, 56 cm; f = 0.5: 0.95, 1.1875, 0.5, 0.6, 0.125, 0.75, 68 cm), `reach_follows_scale_and_encumbrance` (72 cm, 98 cm), `maximum_health_by_race_byte` (every byte rule, manipulator 0.375), `health_from_the_saved_pools` (0.125/0.25; unsaved full; corpse empty), `body_rule_bits` (0x12, 0x2, 0x8, 0xC, 0x67, 0x7, 0x140, 0x170/0x160; with a core 0x70 and 0x40), `voice` (0.9, 1.07, 0.85, 2.025, −1), `default_is_a_new_record_of_no_race`
- [x] 3.2 Unit (`exa_physics`): `energy_recovers_to_its_cap` (a zombie's 0.9 on its first tick, then +0.005), `pose_gains_are_the_originals` (head 0.0052 half feral, none at 1), `feral_arms_hang_loose_striding` (0.05, 0.125; 0.2 dazed, 0.5 off balance), `part_factors_and_arm_strengths` (head 0.025 and 0.075; σ 0.297 off balance at scale 0.95)
- [x] 3.3 Corpus (skipped without `EXANIMA_DIR`) `shipped_level_characters`: 477 placed, 451 living all at full health; scale, size and reach by height and feral level for each; the 97 humans' most energy, pins, head gain, walking arms and sprint by feral level, scales 0.95 (×3) to 1.05 (×4), reaches 54.6 to 70.4 cm, rule bits 0xC ×68 and 0x8 ×29; the other races' counts, maximum health and rule bits by race byte
- [x] 3.4 Corpus `shipped_arena_actors`: 21 actors, every opponent and the proctor (pools 0/0) full; the demon 1.25, scale 1.5, pins 1.3, bits 0x65; opponent04 scale 1.05 and reach 82 cm; arena humans' bits 0x2
- [x] 3.5 `exa-tool characters exanima01` and `characters actors` against the installation (Zombie01 most energy 0.9, head gain 0, health 0.25 of 0.25)

## 4. Verification

- [x] 4.1 REA and `exe.asm`: the builder's clamps and scale/size/height writes, the init (every derived field, its race-byte blocks, the last writes), the head setup, the walking-arm rule, the energy cap, the spawn's call order (RB51), the placement reader's refill, the rule-bit readers (RB49 in part); findings and corrections in `context/re/master-body.md` §18
- [x] 4.2 In game (light runs, `--no-gi --no-supersample --size 960x540`, `--no-shadows` the second time): exanima01's 32 living NPCs stand idle on their derived stats for the 202 body ticks before the GPU ran out of memory while dressing (gnome-shell holds ~15.8 GB of the RTX 4000); no screenshot

## 5. Documentation

- [x] 5.1 `docs/formats/RFC.md` (stats derived at spawn), `docs/formats/RSQ.md` (most energy, head gain and factor, walking arms, limbs' scale), `docs/PLAN.md` (Phase 3), crate READMEs (`exa_formats`, `exa_physics`, `exa_combat`, `exa_world`, `exa_tool`)
- [x] 5.2 `openspec validate --all --strict`
- [x] 5.3 `make check` equivalent on the `play` profile: `cargo fmt`, `cargo clippy --profile play --workspace --all-targets -D warnings`, `cargo test --profile play --workspace` with `EXANIMA_DIR`

## 6. Merge onto main

- [x] 6.1 Rebased onto main (weapon record, strike with fists, clip starts, stance and daze starts): `Fighter::of_stats` wherever main built fighters from a race or a health factor (the placed characters, tests); `Drive::max_energy` and `Drive::energy_share`; the daze test, the pose hold, the swing clock and the attack gates of the player and the duellist read the character's most energy; tests `a_feral_body_is_dazed_under_its_own_most`, `a_feral_attack_waits_for_80_percent_of_its_most`, the zombie's swing clock in `clocks_run_at_the_originals_rates`
