## 1. The word

- [x] 1.1 `exa_combat::condition`: `Condition` with the original's values and masks (`helpless` 0xAC0, `fallen_or_dying` 0xA00, `off_feet` 0x200, `dying` 0x800, `busy` 0x10203, `blocks_daze`), its writers (`daze`, `get_up`, `clip_ended`, `die`); `StateFlags` (dormant 0x10, waking 0x20, held 0x40, shock stagger 0x100, potion 0x800000)
- [x] 1.2 `posture(condition, &Posture) -> (Condition, Branch)`: collapsing/dead kept, shocked until its timer, getting up kept, under 0.6 (double) down below 0.3 (lying still under 25 cm of summed limb motion, else falling), an attack 2, tone under 0.85 weak, the kick, parry or combat idle, powers, walk or run, weapon switch (past 0.6, double), else normal
- [x] 1.3 `PostureTick` (after `StanceTick`, before `CharacterTicks`) and `track_condition`: the daze's writer, then the decision from the clip playing and last tick's balance, `Ragdoll::exertion` and tone; `ConditionPlugin` in `CombatPlugin`
- [x] 1.4 `exa_combat::fighter`: `Fighter` (requires `Condition` and `StateFlags`, no `down`) and `Health`/`DEATH` moved; `damage` re-exports them
- [x] 1.5 `exa_physics::body::Ragdoll::exertion`: each part's farthest particle over the last tick, summed, in cm

## 2. Writers and readers

- [x] 2.1 Writers: `balance::get_up` (getting up), the character tick's clip end of a clip played once (getting up → normal), `react_to_hits` (dead on a killing blow), `viewer` (placed corpses dead)
- [x] 2.2 Readers: hit detection (skips the dead), `take_stance` (skips the dying; the daze leaves a helpless or attacking clip alone), the player's steering, combat facing and attack request (stop while helpless), its target (any not dead), the duellist (idles for 0xA00, leaves a dying target), the world cursor (`cursor::Condition::of`), the body's turn (0 while helpless)
- [x] 2.3 `--trace`: the last column, the condition word in hex

## 3. Tests

- [x] 3.1 Unit (`condition_test.rs`): `the_masks_hold_the_originals_values`, `standing_the_clip_decides` (13 clips, the switch at 0.599/0.6f compared in double, parry, unknown), `weak_under_a_tone_of_0_85` (0.84, 0.85, attack at 0.1), `off_balance_the_body_falls_then_lies_down` (0.6f stands, 0.599, 0.3f, 0.299 with 24.9/25 cm), `getting_up_lasts_until_the_clip_ends`, `collapsing_dead_and_shocked_skip_the_decision`, `the_writers`, `the_posture_stage_writes_once_a_tick` (1, 8, 2, a dazed attacker stays 2, 0, dead kept, limp left)
- [x] 3.2 Unit: `fighter::a_fighter_keeps_a_condition`, `exertion_sums_each_parts_farthest_particle` (2 cm, 3.236 cm), the cursor's `the_condition_word_picks_the_cursors_condition`, the trace's condition column (`0x300`), the reactions and stance tests on the word (normal after a wound, dead after a killing blow)
- [x] 3.3 Corpus (skipped without `EXANIMA_DIR`): `balance::the_condition_reads_the_installed_actions`: the action indices the decision tells apart (0x00, 0x01, 0x04, 0x05–0x0C, 0x0D, 0x1F, 0x22–0x28) are the installed `humanoid.rsq`'s (also printed by `exa-tool rsq`)

## 4. Verification

- [x] 4.1 REA and `exe.asm`: the posture block (branches, constants 0.6/0.3 double, 25, 0.85 float from the image, 0.6 switch), the swing routine's 2, the daze block's 0, the get-up start's 0x300, the clip end's 0x300 → 0, the walk stop's 1 → 0, the collapse start, the dying tick and death, the readers' masks; findings and corrections in `context/re/master-body.md` §20
- [x] 4.2 In game (light runs on the RTX 4000, `--no-gi --no-supersample --size 960x540`): `--shove 60 --trace play arena`: 0 in Idle and the daze's Defence stance (tick 139), 0 while Fall Backward starts at balance 0.53 (tick 161), 0x200 from balance 0.25 (tick 170), 0x300 at `Get Up 2`'s start (tick 369), 0 when it ends into Idle (tick 524); `--opponent --trace play arena`: 8 in Cmbt - Idle, 0 in each daze's Defence stance, 0xA00 at the killing blow (tick 1217)

## 5. Documentation

- [x] 5.1 `docs/formats/RSQ.md` (Condition, the trace's column), `docs/UI.md` (the cursor reads the word), `docs/PLAYER.md` (steering and the daze), `docs/DEBUGGING.md` (the trace), `docs/PLAN.md` (Phase 3), crate READMEs (`exa_combat`, `exa_game`, `exa_physics`)
- [x] 5.2 `openspec validate --all --strict`
- [x] 5.3 `make check` equivalent on the `play` profile: `cargo fmt`, `cargo clippy --profile play --workspace --all-targets -D warnings`, `cargo test --profile play --workspace` with `EXANIMA_DIR`
