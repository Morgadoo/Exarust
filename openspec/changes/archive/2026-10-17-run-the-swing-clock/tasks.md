## 1. The swing routine

- [x] 1.1 `exa_combat::swing`: `Swing` with the record's recoil (W17) and handling (W16); the rate in the original's precision with the stab's shaping and the feral ramp; tiredness (`paced`, double-precision 0.8); the expected swing time; the release stall and Feint's renewal; the chain blend; the attack turn rate
- [x] 1.2 `SwingState` (state word, P, stall, stall end, renewal, rate, strikers' window and trackers), required by `Fighter`; `start`, `chain`, `release`, `daze_stall`, `tick` in the original's order; the trackers
- [x] 1.3 `run_swings` in `FixedUpdate` after the posture stage and before the character ticks (replaces the stamp of the last attack tick): stamp, tick with the strikers' tip velocities, Remise chain (the fighter's pick, chain blend, carried window), stall end → stance (2·W19); off balance (under 0.6) no routine and the clip at 1.25 × base; `limp.rs` reads the rate for an attack's clip

## 2. The executor and the player

- [x] 2.1 `exa_combat::attack`: `Busy` and `Start`, the busy path (the released overhead within 300 ms, Feint's stab/overhead with blend 10 or side attack) and the free path's released overhead and Feint's waived cooldown; `Side`, the picks on `Attacker`, the side attack's start time; `is_striker`; `start_blend` takes the executor's blend
- [x] 2.2 `start_attack` takes the swing state and an `AttackStart` (motion, blend, weapon class, Remise, now): fresh swing state, side start stamped
- [x] 2.3 `exa_game::player`: the release when the request drops during an attack, the state word kept outside attacks, the released main-side attack's return to the stance (2·W19, 30 without a primary), the executor's state, Remise and Feint from the fighter's perks; the attack turn while attacking
- [x] 2.4 `exa_ai`: the duellist through the new executor and start (it never releases)
- [x] 2.5 `Fighter::perks` (`with_perks`, `remise`, `feint`); placed characters keep their records' perks

## 3. Hits and blows

- [x] 3.1 Detection by open windows (held items; fists only when held), the swing's clock for the blow
- [x] 3.2 The struck attacker's stall from the daze (D capped at 0.9, f = 0.25 over the most health, halved under a potion) replaces the stop-at-once stand-in; a killing blow still ends the attack; `exa_physics::body::MAX_DAZE` public

## 4. Debug helpers

- [x] 4.1 `--perks a,b,...` (names, case aside; an unknown name stops the launch)
- [x] 4.2 `--trace` swing columns (`swing_rate`, `strength`, `attack_flags`); logs for releases, stance returns, chains, struck swings

## 5. Tests

- [x] 5.1 `swing`: the sword 0.87/1.30/0.97, Sword A 1.055/1.107/1.266 and the fists 1.62 (68 ticks, 816 ms for 110 frames); the stab; the feral ramp ×0.5/×1/×1.5; tiredness; the expected time 849/1053/426/802 ms; the release stall 0.3/0.15/0.0714/0.0357/0.01 and renewal 110/220/330, 142/284/426, 161 ms; a release's end 42/175 ms and the capped 426; the daze stall 1.2 (67 ms), 0.632 (127 ms), none for a fast swing or a spent one, the cap 1, the second factor 1/0.5/0.5; windows by motion and class, the overhead's and stab's closes, Remise past 0.8 (and its exclusions), the carried window, the stall's drain and end, the trackers, the attack turn 0.0404 rad, the chain blend 2/4/3, the off-balance rate 1.25/1.875 (none at 0.6); the routine's stamp, chain and stance in an app
- [x] 5.2 `attack`: the executor's waits, the released overhead within 300 ms, Feint's stab/overhead with blend 10 and side attacks, busy while struck, Feint's waived cooldown; the picks (moved from the player), the blends
- [x] 5.3 `exa_combat`: a forearm cut mid-punch (daze 0.428) stalls by 1.713, stance due 47 ms later, attack and clip kept; no stall when not attacking; fists strike with the motion's hand through the window
- [x] 5.4 `player`: a click checks the punch (stall 0.3, stance at 12 054 ms, renewal 12 122, back on the 5th tick, blend 0, state word kept released); a held button never stalls; a tap then a press 216 ms after the start throws the overhead before the cooldown; Feint renews a released punch, without it the cooldown holds
- [x] 5.5 `limp`: an attack's clip runs at the routine's rate; `trace`: the swing columns; `exarust`: `--perks` parsing
- [x] 5.6 Corpus: the workspace tests with `EXANIMA_DIR` (no corpus rule changes in this change; the records' perks are covered by `characters` corpus tests)

## 6. Verification

- [x] 6.1 Light runs through `gpu_run.sh --gpu 4000` (`--no-gi --no-supersample --size 960x540`, arenao1, `--opponent --trace`), no panic:
  - `--press LMB@200-420,LMB@520,LMB@640,LMB@652,LMB@800-1100`: held punches 67 attack ticks each (804 ms + the ending tick; 336 ms cooldown between), clicks released the next tick (stall 0.3) and back in the stance after 5 ticks, the double click's overhead 204 ms after the punch's start ("after a release"), struck swings stalled (1.543, 1.983, 1.791 a tick), a tired punch (E 0.80, rate 0.647) 87 ticks
  - `--perks remise,feint --press LMB@200-600,LMB@700,LMB@760-900`: Attack Down chains Combo 2 after 55 ticks (blend 2), a tired Attack Up after 73; a swing struck at 0.71 stalls (0.11 a tick) to its expected end; releases at 0.55 and 0.84 with tracked speed (stall 0.243, 0.253)
  - `--weapon "sword gsf 01" --perks remise --press LMB@200-800`: Greatsword (cooldown 474 ms, W15 1.086, W18 0.299 from the trace) Attack Mid chains after 78 ticks (936 ms; the formula: 78), combo blend 4; the same weapon on a 100-frame clip would chain after 71 ticks (852 ms) against the live trace's 828 ms (69 ticks) for the player's two-hander
- [x] 6.2 Remove probes and scratch files from the tracked tree (none were added)

## 7. Documentation

- [x] 7.1 `docs/PLAYER.md` (Combat mode), `docs/formats/RSQ.md` (Motion playback, the trace's swing columns), `docs/DEBUGGING.md` (`--perks`, the logs, the columns), `docs/PLAN.md` (Phase 4), crate READMEs
- [x] 7.2 `openspec validate --all --strict`
- [x] 7.3 `make check` equivalent on the `play` profile: `cargo fmt`, `cargo clippy --profile play --workspace --all-targets -D warnings`, `cargo test --profile play --workspace` with `EXANIMA_DIR`
