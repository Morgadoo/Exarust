## 1. Bleeding blows

- [x] 1.1 `exa_combat::damage`: `Outcome::wound` (R = P/(P + 1)·m, before the red bar's 0.75; red = 0.75·R)
- [x] 1.2 `exa_combat`: `Fighter::bleeds` (true by default), `Fighter::of_record` (health, natural armour and bleeding from a `races.rdb` record); placed characters built from their record in `exarust`
- [x] 1.3 `Hit::striker` (the blade entity) set by the hit detection; `CombatSystems` set for the combat's frame systems

## 2. Wound queue

- [x] 2.1 `exa_combat::wounds`: `BLEED_FROM` 0.05, `WOUND_SLOTS` 7, `WOUND_DELAY` 7, `bleeds`, `wound_radius` (30·R + 10 cm), `frame_tetra`, `tetra_weights`, `tetra_point`; `WoundQueue` (required by `Fighter`) with `push` (dropped when seven wait) and `tick` (all count down, the first out when due)
- [x] 2.2 The blow reactions queue a wound when the fighter bleeds and R > 0.05, with the striker's weights at the blow; the hit log names the struck body, the body tick and R
- [x] 2.3 `count_down_wounds`, first in `CombatSystems`: one count per body tick since the last frame, a `Bleed` per due wound (travel from the striker's weights now, radius)
- [x] 2.4 Tests: `wound_sizes` (R widened to double: 0.049 999 996 no, the single-precision 0.05 and 0.0501 yes; 11.5, 19, 40 cm), `weights_ride_with_the_striker`, `the_queue_waits_seven_ticks_one_a_tick`, `a_wound_bleeds_seven_ticks_after_the_blow` (R 0.457, 23.7 cm, nothing for six ticks, the travel of the turned blade), `skeletons_do_not_bleed`, `shallow_wounds_do_not_bleed` (hand at 4 cm/tick R 0.045 no, 6 cm/tick R 0.064 with red 0.048 yes), `the_fallen_bleed_and_seven_wait_at_most` (killing blow on the corpse; nine cuts, seven painted ticks 7–13), `fists_stay_clean`

## 3. Painting

- [x] 3.1 `exa_character::blood`: `bleed_wounds` turns each `Bleed` into a `PaintWound`, after `CombatSystems` and before `paint_wounds`
- [x] 3.2 Test `hits_bleed_seven_ticks_later`: a cut through the combat and blood plugins paints nothing for six ticks, then on the seventh soaks the strip ahead of the centre (10 cm behind the contact, the blade moved 20 cm along +x) and leaves a ball of radius 30·R + 10 cm behind; the striker's body, the helmet and the sword stay clean

## 4. Debug helper

- [x] 4.1 `exarust --click frame,...` (left mouse button clicks at frames after the load; a bad value prints the form and exits with code 2); test in `scripted_presses_are_read`

## 5. Verification

- [x] 5.1 REA and `exe.asm`: the blood branch (race flag bit 0, R before the 0.75 against 0.05 as a double, strictly; the queue call with the striking body and R; the striking-hand slot), the queue (seven records, full → dropped, delay 7, weights from the striking body's particles), the consumer (every delay counted down, the first painted when ≤ 0, D = weighted point now − contact, radius (30·R + 10)² from 30.0 and 10.0 floats), its single caller in the body tick; notes appended to `context/re/wounds-and-blood.md` §9
- [x] 5.2 Corpus: `races.rdb` flag words stay pinned by `shipped_races` (with `EXANIMA_DIR`)
- [x] 5.3 In game through `gpu_run.sh --gpu 4000` (`--opponent play arena`, and `play arena1` with `--weapon ub05ssworda --press Tab@30 --click …` so both strike) (logs and shots in the scratchpad, no panic or validation error): arena duels; 46 bleeding wounds painted a median 84 ms after their blow; blows at or under R 0.05 left no blood; the player bleeds at the struck head, neck, chest and shoulder after two blows and on its corpse; blood pixels median sRGB (64, 31, 26) on the player and (51, 25, 22) on the corpse against the original's (54, 25, 21) and (47, 24, 20); the duellist bled on its shoulder from the player's chest cut (R 0.054, 12 cm, 7 meshes copied, 229 KiB); its `helm_b` stopped all 13 of the player's head blows and 2 of the duellist's on a helmeted player (penetration 0, R 0), so no helmet blood from hits (the helmet takes blood from wounds that reach it: tests and `shade-and-paint-blood`); a player in the duellist's outfit bled on skin and jacket (66 meshes copied, 1.1 MiB, after three wounds); 60 FPS on a bloodied corpse, as the previous build without blood
- [x] 5.4 Remove the probes from the tracked tree

## 6. Documentation

- [x] 6.1 `docs/RENDERING.md` (Blood: wounds from hits), `docs/formats/OTHER.md` (Damage: bleeding), `docs/DEBUGGING.md` (`--click`, the hit log), `docs/PLAN.md` (Phase 5), READMEs of `exa_combat`, `exa_character`, `exarust`
- [x] 6.2 `openspec validate --all --strict`
- [x] 6.3 `make check` equivalent on the `play` profile: `cargo fmt`, `cargo clippy --profile play --workspace --all-targets -D warnings`, `cargo test --profile play --workspace` with `EXANIMA_DIR`
