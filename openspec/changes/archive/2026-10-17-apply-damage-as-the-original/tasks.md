## 1. The damage call

- [x] 1.1 `exa_combat::fighter`: `Damage { yellow, red, vital }` (`Damage::blow`: a weapon blow's, vital), `Taken { Hurt, Weakened, Killed }`, `DEATH` 0.01 as a double, `LOW_HEALTH` 0.05, `failing(yellow)` (the double test)
- [x] 1.2 `Health::take(damage, rules, flags, story) -> Taken` in the original's order and precision: rate 0; yellow −0.1·Y (−0.05·Y with `StateFlags::POTION`) in double; rule bit 0x8 → yellow −0.2·R, red kept; else red −0.2·R in double and yellow = red_new / red_old × yellow in float (0 when red ≤ 0, red not clamped); yellow ≥ 0.05 → `Hurt`; not failing → `Weakened`; else yellow 0, not vital → `Weakened`, else in the story red −0.1 × max in double, `Killed`
- [x] 1.3 `StoryMode` resource; `Weakened { fighter }` message, registered by `CombatPlugin`
- [x] 1.4 `react_to_hits`: `take` with the victim's `stats.rules`, its `StateFlags` and `StoryMode` (absent: not the story); `Weakened` written for `Taken::Weakened`; the collapse on `Taken::Killed` as before
- [x] 1.5 `stance::player_stance`: Defence while `failing(yellow)` (0.01 in single precision included)

## 2. Heartbeat and viewer

- [x] 2.1 `exa_game::hud::heartbeat`: started by a `Weakened` on the player's body (still once a minute, story levels or `--heartbeat`), no longer by the HUD's yellow falling into 0.01…0.05; `starts_heartbeat(since)`; `LOW_HEALTH` and the yellow memory removed from the HUD
- [x] 2.2 `exarust`: `StoryMode` inserted for story levels (`exa_world::start::mode`); `--feral f` (debug) gives the player's record feral level f, logged with its rule bits and most energy

## 3. Tests

- [x] 3.1 Unit (`fighter_test.rs`): `the_red_share_scales_the_cut_yellow` (0.09 / 0.15, rate 0), `no_lasting_wounds_take_red_damage_from_yellow` (feral 0xC and 0x8: 0.14 / 0.25; human 0.152 / 0.19), `a_potion_halves_the_yellow_damage` (0.17099999 / 0.19), `no_red_left_empties_yellow` (red −0.15, killed), `the_lines_at_0_05_and_0_01` (0.01f killed, 0.010000001 weakened, 0.05 hurt, 0.049999997 weakened), `the_subtractions_are_in_double_precision` (Y 2 from full: 0.05, hurt; 0.049999997 in single precision), `non_vital_damage_never_kills` (yellow 0, red 0.2 in the story), `a_killing_blow_in_the_story_costs_red` (0.175 / 0.2), `health_and_death`
- [x] 3.2 Unit (hit reaction, `lib.rs`): `the_victims_rule_bits_decide_the_damage` (a feral 0.5 record's rule bits 0x8: red 0.25 after the cut), `a_low_blow_weakens_the_fighter` (the message only for the blow leaving 0.03, none at full health or for the kill), `a_killing_cut_in_the_story_costs_red`; `stance::three_seconds_after_an_attack_and_dying` (0.01 → Defence); `regen::a_blow_restarts_the_ramp`
- [x] 3.3 Unit (`exa_game::hud`): `the_players_low_health_check_starts_the_heartbeat` (the player's body starts it, another fighter's does not, nor the arena), `a_low_blow_starts_the_heartbeat_once_a_minute`
- [x] 3.4 Corpus (skipped without `EXANIMA_DIR`; `exa_rpk` a dev-dependency of `exa_combat`): `shipped_characters_take_damage_by_their_rule_bits`: Y 0.5 and R 0.3 on every living character of the story levels keep the red of 248 of the 451 (97 feral humans, 128 skeletons, 23 ancients, exactly those with rule bit 0x8) and take 0.06 from the other 203

## 4. Verification

- [x] 4.1 `exe.asm`, REA cross-references and the PE: the damage call's order and branches, 0.1 / 0.05 / 0.2 doubles, 0.05 float (no writer), 0.01 double, the never-written "no death" byte (RB48) and never-set player hook, the story byte's writers (modes 2, 4, 8), the two callers' vital arguments, the low-health check, the three readers of yellow under 0.01; findings in `context/re/master-body.md` §23
- [x] 4.2 In game (light runs on the RTX 4000, `--no-gi --no-supersample --size 960x540`): `--feral 0.5 --opponent play arena`: rule bits 0x8, most energy 0.95, five cuts leave yellow 0.213, 0.155, 0.081, 0.022, 0 with red 0.250 throughout, then the collapse and death 250 ticks later; `--heartbeat --opponent --health 0.06,0.25 play arena`: the first cut (yellow 0.041, red 0.224) starts the heartbeat on the next frame, the second (0.014) none within the minute

## 5. Documentation

- [x] 5.1 `docs/formats/OTHER.md` (Applying damage), `docs/formats/RFC.md` (rule bit 0x8), `docs/UI.md` (Heartbeat), `docs/PLAYER.md` (the dying stance), `docs/DEBUGGING.md` (`--feral`), `docs/PLAN.md` (Phase 4), `crates/exa_combat/README.md`, `crates/exa_game/README.md`
- [x] 5.2 `openspec validate --all --strict`
- [x] 5.3 `make check` equivalent on the `play` profile: `cargo fmt`, `cargo clippy --profile play --workspace --all-targets -D warnings`, `cargo test --profile play --workspace` with `EXANIMA_DIR`
