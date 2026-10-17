## 1. The fall

- [x] 1.1 `exa_combat::fall`: `FALLING_BELOW` (−5 cm a tick), `LETHAL_DROP` (2,500 cm), `PIT_AFTER` (1,000 ms), `PLAYER_DIES_AFTER` (4,000), `DIES_AFTER` (3,000); `FallStage` (steady, falling, lost, fading, gone), `FallDeath` (stage, `since`, `out`), `FallReading` (metres; `lethal`: pelvis − ground ≥ 2,500 cm under the body and every foot, none fails), `FallDeath::check` (the original's order and strict comparisons), `FallDeath::fall_out` (once), `scream_voice` (pitch from the first draw, gain from the second, double precision)
- [x] 1.2 `condition::PostureBranch`, written by `track_condition`; `Fighter` requires it and `FallDeath`; `Roll` public
- [x] 1.3 `exa_physics`: `Balance::ground`, the Height Query under the pelvis without support (+20 cm, r 20 cm, 2,000 cm, −10,000 cm on a miss), the get-up support test reading it
- [x] 1.4 `exa_game::fall`: `fall_to_death` (posture stage, after `track_condition`, living bodies in the falls branch; scream sound, pit sound, `DeathFade` start, death and removal of the character's root) and `fall_out_of_world` (after `BodySystems`; dead bodies that fell out, once; the player's kept); `FallPlugin` in `GamePlugin`; tick-stamped info lines
- [x] 1.5 `exa_ui::fade`: `DeathFade`, `opacity` (3 s, round half to even), the black layer at Z 895 on virtual time, hidden while loading

## 2. Tests

- [x] 2.1 Unit (`exa_combat` `fall_test.rs`): `it_screams_once_past_5_cm_a_tick` (−5.0 steady, −5.01 screams, −4 steady again, NaN steady), `the_dead_do_not_scream`, `twenty_five_metres_over_every_ground` (25.00 m lost, 24.99 m not, a foot 2 m down or out of reach not, supported never, no feet), `the_timelines` (84, 334, 251 ticks), `lost_is_lost` (3,000 ms not yet, 3,012 dies), `the_kill_plane_takes_a_body_once`, `the_screams_gain_and_pitch` (0.2/0.95, 0.35/0.95, 0.275/1.05)
- [x] 2.2 Unit (`exa_game` `fall_test.rs`): `a_character_dies_3_seconds_after_it_is_lost` (a biped sunk 8 cm a tick over nothing: lost on the first check, alive on the 250th tick after, its root despawned on the 251st), `the_player_fades_then_dies_4_seconds_after` (fade at 84, alive at 333, gone at 334), `only_the_falls_branch_checks`, `the_kill_plane` (a corpse taken, the player's corpse kept with health 0, the living and the lost left, never twice)
- [x] 2.3 Unit (`exa_physics`): `without_support_the_ground_under_the_pelvis` (15 m and 18.5 m found, 19 m −100 m, standing none), `balance_is_0_without_ground` (ground −100 m); (`exa_ui`) `the_fade_takes_3_seconds` (0, 1, 128, 254, 255)
- [x] 2.4 Corpus (skipped without `EXANIMA_DIR`): `the_installed_fall_sounds` (`pitdeath01` and `vhm01`–`vhm09` `_groanh01`–`06` in the installed `Sound.rpk`, no `vhm10`, no seventh groan; also listed by `exa-tool ls Sound`)

## 3. Verification

- [x] 3.1 `exe.asm` and the PE: the fall check's constants and order, the two call sites (the dead one unreachable), the pit routine (gain 0.1, flags 0x4060), the voice slots (kind 2 = `_GroanH`), the session hook and the HUD's fade (3 s, then a pause), the camera nudge's field (the wall tilt), the character ground's query, the kill plane (plain step only, the callback), body flag 0x80000's readers; findings in `context/re/m44-notes.md` (§24)
- [x] 3.2 In game (light runs on the RTX 4000, `--no-gi --no-supersample --size 960x540 --at 0,200,0 play arena`): balance lost on the first ticks, scream and loss on tick 89 at 199.34 m, pit and fade on tick 173, death and removal on tick 423 ("4008 ms"); `--pause-at-tick 150` and `300`: the body's brightest pixel 79 unfaded, 42 at 1.5 s into the fade

## 4. Documentation

- [x] 4.1 `docs/formats/RSQ.md` (Falling to death; Condition's readers), `docs/UI.md` (Death fade), `docs/PLAYER.md` (Falling), `docs/DEBUGGING.md` (the fall's log lines, a drop test), `docs/PLAN.md` (Phase 4), READMEs of `exa_combat`, `exa_game`, `exa_ui`, `exa_physics`
- [x] 4.2 `openspec validate --all --strict`
- [x] 4.3 `make check` equivalent on the `play` profile: `cargo fmt`, `cargo clippy --profile play --workspace --all-targets -D warnings`, `cargo test --profile play --workspace` with `EXANIMA_DIR`
