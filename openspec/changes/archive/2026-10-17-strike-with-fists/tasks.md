## 1. Race weaponry

- [x] 1.1 `exa_formats::weapon_record`: `RaceWeapons::of_stats` from the decoded race stats (unarmed reach m → cm, swing turn, recovery, kind word); the second reader of the `races.rdb` payload removed; the corpus test walks `races::parse`
- [x] 1.2 `exa_character::RaceWeaponry` (race index and fields, the human's by default) on placed characters and the character view's race; held weapons held by it

## 2. Fists as strikers

- [x] 2.1 `exa_assets::ModelNode::half_extents` and `exa_world::PartBounds::half_extents` (the RFC node box)
- [x] 2.2 Dressing an unarmed fighter: both hands get a `Blade` (right first, left second) from the race's fist or claw records across the hand's box, turned along the hand's x toward the box; equipping the fists clears the primary striker, a weapon becomes it
- [x] 2.3 `exa_combat`: `Blade::slot`; `Attacking` keeps the motion and the weapon class; contacts only from the strikers the attack strikes with (`attack::strikes`); a fist's contact 0.1 cm inside its far end; stab and overhead from the motion; the hit log names a fist

## 3. The executor

- [x] 3.1 `exa_combat::attack`: the cooldown (400 ms, round(300 × recoil) + 300), the 0.8 energy gate in double precision, the stab's facing gate, `may_thrust`, the motions and their clips, the striking hand and the primary slot, `execute`, `Attacker` (last attack tick, primary), the stamp of the last attack tick each fixed tick before the character ticks; `Fighter` requires `Attacker`; `start_attack` sets the primary
- [x] 3.2 `exa_physics::body`: `BodyTicks::millis` (the 12 ms game clock), `MAX_ENERGY` public

## 4. The player and the duellist

- [x] 4.1 `exa_game::player`: the request each behaviour tick (press, double click on the game clock, the side at the press, hold, release, Alt), the heading to the cursor while it stands, the energy gate and the executor, the picks (crouched → Attack Mid); a latch for presses between ticks; the per-frame facing reads the request; logs of each attack and of a request waiting for energy
- [x] 4.2 `exa_ai`: the duellist's strikes through the energy gate and the executor's cooldown, its weapon class from its strikers, its reach without fists

## 5. Debug helper

- [x] 5.1 `--pause-on-hit <n>`: pause n ticks after the player's first blow lands (`SimControl::with_pause_on_hit`), screenshots counted from the pause

## 6. Tests

- [x] 6.1 `attack`: 400/330/426/425/412 ms cooldowns and the 28th tick for 330 ms; the energy gate at 0.8/0.799 of 1 and 0.76/0.759 of 0.95; the stab gate at 0, −0.36, −0.37; the executor's waits and exclusions; the striking hand per motion and class; the primary slot; motions and clips; the stamp every attack tick and not after; the cooldown from the fighter's own primary blade
- [x] 6.2 `exa_combat`: fists strike with the motion's hand (main side and overhead the right, other side the left), the knuckles at 17.56 of 17.66 cm, the blow resolving with damage; no fist strikes with a weapon's class
- [x] 6.3 `exa_character`: human fists 17.66 cm, recoil 0.1, swing speed 1.44, blend 0, the striker's direction; the Harvester's right fist and left heavy claw (impact 0.83); corpus: the human body model's hands through the spawned bounds, 18 races' weaponry (Sir 150 cm)
- [x] 6.4 `player`: the request (double click at 299 ms, both presses used, the side at the press, release); hold to repeat (Attack Up, nothing while it plays, nothing at 13 320 ms, the next at 13 332 ms after a last tick at 13 000, nothing after release); the left fist's other side after a 400 ms first cooldown; no attack at energy 0.79, one at 0.8; a click between ticks; crouched picks
- [x] 6.5 Corpus (skipped without `EXANIMA_DIR`): `weapon_record` fists and claws of every race from `races::parse`
- [x] 6.6 `sim_control`: `--pause-on-hit 3` ignores the duellist's blow, pauses three ticks after the player's first and is not moved by a later one

## 7. Verification

- [x] 7.1 Light runs through `gpu_run.sh --gpu 4000` (`--no-gi --no-supersample --size 960x540`), no panic or validation error:
  - `--opponent --camera 4.2,1.4,0.6,90,-8 --press LMB@150-1500 play arenao1`: the first punch with a 400 ms cooldown, then each next one 336 ms (28 ticks) after the last attack tick; after sword blows "attack waits: energy 0.58 / 0.51 / 0.60", the next punch at energy 0.80; from 1.2 m the punches mostly fall short (one fist hit on the duellist's hand)
  - the same with `W@100-1500` (stepping in): 11 fist hits on the duellist (chest, arms), 16–18 cm along the fist, 2.0–2.9 cm/tick, impact up to 0.33, its yellow health from 0.25 to 0.035
  - `--still-opponent --at 0,0,-1.1 --camera 4.2,1.4,-1.0,90,-8 --press LMB@120 --pause-on-hit 3`: the right straight lands on the duellist's arm (bone 12, 16 cm, impact 0.26), the screenshot shows the right arm fully out, the body twisted into it and the left arm back, as reference shot 4
  - `--opponent --weapon ub05ssworda --press LMB@150-1300`: the sword's 426 ms cooldown from the first attack, sword hits on the duellist, no fist hits
- [x] 7.2 Remove probes and scratch files from the tracked tree (none were added)

## 8. Documentation

- [x] 8.1 `docs/PLAYER.md` (Combat mode), `docs/DEBUGGING.md` (`--pause-on-hit`, the attack examples, the logs), `docs/formats/OTHER.md` (Weapon record: fists as strikers, race fields from the stats), `docs/PLAN.md` (Phase 4), crate READMEs
- [x] 8.2 `openspec validate --all --strict`
- [x] 8.3 `make check` equivalent on the `play` profile: `cargo fmt`, `cargo clippy --profile play --workspace --all-targets -D warnings`, `cargo test --profile play --workspace` with `EXANIMA_DIR` (481 passed, 3 ignored)
