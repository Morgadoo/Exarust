## 1. The stance

- [x] 1.1 `exa_combat::stance`: `Stance` (Idle, Defence stance, Cmbt - Idle), the player's rule (`player_stance`: Idle out of combat; Defence while yellow < 0.01; Cmbt - Idle with the target nearer than 3.5 m × its scale or within 3 s of the last attack tick; else Defence), the target pick (`pick_target`: within 5 m, best (5 − d)(cos + 0.5) > 0), the duellist's rule (`duellist_stance`: Defence at ≥ 4 reaches), the set-stance helper, the posture switch's test (looping, not the stance, not Cmbt - Idle, Walk, Run, Backpedal or Combat Walk, balance ≥ 0.8, tone ≥ 0.85), readiness (+0.004, cap 1, else 0) and the chest point
- [x] 1.2 The body tick system, in the fixed step before the character ticks: readiness, a clip played once whose follow-up is a stance ending in the fighter's stance (falls keep theirs), the posture switch on living, standing bodies
- [x] 1.3 `Fighter::readiness`; `damage::resolve` scales penetration, impact and stab energy by 1 − 0.5g after taking the impact for the shove; `WeaponRecord::reach` (60·scale + the sweet spot)

## 2. The player

- [x] 2.1 `PlayerBody` requires `Stance`; the player's rule each fixed tick after the steering: the last attack time refreshed while attacking, Idle through the helper out of combat, the target among the other living fighters from the cursor's direction, the stance written in combat; each change logged
- [x] 2.2 Tab only toggles combat mode (`set_combat`, the footwork along the heading); the interim combat walk gives back the clip it replaced, and a walk or run still playing when combat mode turns on ends in the stance; attacks end in the stance; the get-up ends in the fighter's stance (`balance`)

## 3. The duellist and the fight start

- [x] 3.1 `exa_ai::Duellist::new`: idle until engaged; entering combat takes the Defence stance through the helper; then the stance by the chest distance against 4 × its reach (its blades', else 50 cm), started whenever it is free and not walking; strikes end in the stance; `holds_ground` (no steps, no strikes)
- [x] 3.2 `exarust`: the duellist spawns idle; at the end of the load every duellist engages and, in practice and arena levels (not the hub), the player's combat mode turns on; `--still-opponent`

## 4. Tests

- [x] 4.1 `stance`: the player's rule at 3.88/3.83/3.51/3.5 m (Defence) and 3.49/3.0/0.9 m (Cmbt - Idle), twice the scale at 6.9 m, Idle out of combat; 3 s after an attack (0, 1.5, 2.99 s ready, 3.0 s not); dying at 0.009 against 0.011; the target pick (behind and 121° off none, 119° picked, beyond 5 m none, 2 m at 60° beats 4 m ahead); the duellist at 4.8/4.79 m with a 1.2 m reach and 2/1.99 m unarmed; the helper; the posture switch (Idle → Defence, Defence → Cmbt - Idle at 0.8/0.85, never from Cmbt - Idle, the walks, below 0.8/0.85, nor in a clip played once); readiness 0.5 after 125 ticks, 1 at the cap, 0 on leaving; the body tick system (Idle → guard → combat idle, combat idle kept, an attack ending in the new stance, a fall keeping its lying clip, readiness 0.04 after 10 ticks, a dead fighter untouched)
- [x] 4.2 `damage`: readiness 1 halves penetration (0.5 three quarters) and keeps the shove, the daze × √0.5, a stab's penetration halved; the test sword's reach 1.176 m (1.776 m at scale 2)
- [x] 4.3 `player`: the stance rule with an opponent at 4, 3.4, 3.6 m, behind, dead, limp, and out of combat; ready for 2.9 s after an attack, not at 3.1 s; the combat walk giving back the guard; the player keeps a stance
- [x] 4.4 `exa_ai`: idle until engaged, the guard on entering, the walk in from 8 m; a still duellist guarding at 8 m and in Cmbt - Idle at 1 m without striking; a strike ending in Cmbt - Idle
- [x] 4.5 Corpus (skipped without `EXANIMA_DIR`): the existing checks that every weapon's moveset resolves the Defence stance and combat idle in the installed `humanoid.rsq` still pass

## 5. Verification

- [x] 5.1 In game through `gpu_run.sh --gpu 4000`, no panic or validation error in any log:
  - `--opponent --camera 5.5,1.3,6.3,90,-6 play arena`: "the fight starts: combat mode on"; the player in the fists-up guard (the duellist 2.5 m behind it is no target), as reference shots 1–2
  - `--opponent play arenao1` (side camera): the guard at 3.96 m, Cmbt - Idle at 3.50 m as the duellist walks in (switching at 3.49–3.50 m in three runs), the duellist Cmbt - Idle from 3.92 m (reach 0.98 m); its hits land at readiness 1.00
  - `--still-opponent --at 0,0,6 / 0,0,1.7 / 0,0,1 play arenao1`: both guarding at 8 m; the duellist in Cmbt - Idle and the player guarding at 3.66 m; both in Cmbt - Idle at 2.96 m
  - `--still-opponent --at 0,0,6 --press LMB@60`: Cmbt - Idle during the punch and 3.0 s after it, then the stance Defence while the clip stays Cmbt - Idle (no switch logged)
  - `--press Tab@800`: Tab turns combat mode off, the player idles with its arms down
  - `--weapon ub05ssworda --still-opponent`: the One Handed guard (sword hand at mid-chest) at 8 m and the low One Handed combat idle at 3 m, compared with reference shot 6 (the blade points up and forward, the grip is L12's)
  - `--cursor 650,280 --press LMB@2,LMB@200,LMB@206`: a left straight (cursor 48° left), then an overhead
- [x] 5.2 Remove probes and scratch files from the tracked tree (none were added)

## 6. Documentation

- [x] 6.1 `docs/PLAYER.md` (Combat mode; The combat stance), `docs/formats/OTHER.md` (Weapon stats: reach; Damage: readiness), `docs/DEBUGGING.md` (`--still-opponent`, the attack examples without Tab, the stance logs), `docs/PLAN.md` (Phase 4), `crates/exa_combat/README.md`, `crates/exa_ai/README.md`, `crates/exa_game/README.md`, `crates/exarust/README.md`
- [x] 6.2 `openspec validate --all --strict`
- [x] 6.3 `make check` equivalent on the `play` profile: `cargo fmt`, `cargo clippy --profile play --workspace --all-targets -D warnings`, `cargo test --profile play --workspace` with `EXANIMA_DIR` (305 passed, 3 ignored)
