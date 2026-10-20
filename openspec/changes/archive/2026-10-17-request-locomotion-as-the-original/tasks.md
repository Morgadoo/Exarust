## 1. Research

- [x] 1.1 Re-read in REA/`exe.asm` the player behaviour tick, the exploration steering, the soft stop, the movement request, the cursor smoother and its follower, the movement decision, the can-locomote gates, the body turn and alignment, the locomotion's latch reads, the sampler's key segment; constants from the PE
- [x] 1.2 Settle RB23 (W + Shift jogs, RMB far + Shift keeps the latch) and RB24 (the quarter-turn block is an object interaction held by the left button); record both and the corrections in `context/re/master-movement.md` §21

## 2. The request and the decision

- [x] 2.1 `exa_physics::body::locomotion`: `Locomotion` (heading, look, cursor, wanted direction, flag word, moving, free walk, goal, slow turn, interim stepping), `Locomotor` (yaw, error, speed, alignment, amount, reach, leg mode, cooldown, turning, turn rate, stop reason, run blend, stop distance), `Locomotion::facing`
- [x] 2.2 `decide`: move vector, leg mode and brake by the stop distances, the gates in the original's order (stepping refusing without returning), the brake below 1 cm a tick and the turn-about, the move or stop branch; `Footing` (energy from the body, balance steady until C15, `balanced_by`)
- [x] 2.3 `turn_rate`, `turn`, `hold_turn` (the table, crouch, ease, keys, √scale, snap), `align_target`/`align`, `locomote` (run blend and stop distance), `count_down`
- [x] 2.4 The capsule executor `capsule_velocity` and `CapsuleGaits` (Exarust's own until C18: today's speeds, the brake drive and the stop on the goal); `Drive::land`, `Balance::land`, `Ragdoll::pelvis_forward`
- [x] 2.5 `exa_game::stages::movement::decide_movement` in the movement stage before the gait floor: speed from the capsule, the decision, the turn (M22's attack law while attacking, held while helpless or fallen), the alignment, the run blend, the capsule's velocity, the model's yaw, the bearing's moving, run and turn

## 3. The player and the duellist

- [x] 3.1 `exa_game::steering`: the request (W, RMB over the view, S, the hold to key segment 0 or 4), Shift's latch, the cursor's direction and follower, the exploration steering (quarter turn after 1 s, standing look, reversal guard, keys and mouse, run and latch, look-ahead, soft stop), the combat branch's soft stop; `key_segment`
- [x] 3.2 `exa_game::player`: the behaviour writes the request; `PlayerBody` keeps the behaviour's state (yaw, heading and run blend move to the locomotion components); the line walk stand-in (hip ray, clamped a radius and a cell short); the gait clip by the decision; combat footwork as the request's stepping; the attack request and combat heading through the request
- [x] 3.3 `limp::bearing`, `stages::posture`, `stages::sampler`, `balance::get_up` read the locomotion components
- [x] 3.4 `exa_ai`: the duellist writes the request (heading, carrot 1.5 m, walk, no slow turn), `CapsuleGaits` 1.2 m/s, its clip by its body's decision
- [x] 3.5 `exarust`: spawn the player and the duellist with `Locomotion::facing`; `--press Shift`

## 4. Debug helpers

- [x] 4.1 `--trace` locomotion columns (`loco_flags`, `moving`, `free_walk`, `yaw`, `heading`, `turn_rate`, `align`, `speed`, `reach`, `stop`)

## 5. Tests

- [x] 5.1 `exa_physics` locomotion: the base turn 240°/s; the table (240/201.6/0 free walk, 120, 300, 210, 272.7, attack, crouch 180); the step, ease, keys 270, snap and wrap; the quarter turn (38 ticks at 0.0252, 90 % after 70 ticks, 28 at 300°/s); free walk never brakes; braking within 50 cm and stopping at 0.4; arriving within 25 cm at 0.5; the stop distance from the run; backwards and the S-brake; the gates (tired, 0.9 float under the double, 0.99, 80/81, 20/21, the 40-tick cooldown); stepping still stumbles; the alignment (0.75, 1, 0.4, 0.7, 0.982, 0.6977); run blend and stop distance (20 ticks, 0.85, 0.835, 0.36, 1.25, 1.5); the capsule (1.4, 3.6, −1.2252, 0.97397, footwork, braking stops on the goal in 29 ticks within 3 cm, a run stops before a wall); the pelvis forward; the footing
- [x] 5.2 `exa_game` steering: the request, the release hold, the cursor mode, the follower (0.0172718 first tick, settles), the quarter turn after 1000 ms (A, D, both, held object), steering stamps, the standing look (56.31°), Shift's latch (W jogs, far mouse keeps it), the look-ahead (92.1 cm, wall clamp, run cut, −70), the reversal, the soft stop (33.52 cm), no steering while down, combat soft stop, key segments
- [x] 5.3 `exa_game` movement stage: free walk drives the capsule and turns the body; helpless holds; 300°/s standing and the swing's law (168°/s); gaits and footwork; player tests through the decision; trace columns
- [x] 5.4 `exa_ai`: the duellist's request, carrot and stance while held; `exarust`: `Shift` parsed
- [x] 5.5 Corpus (skipped without `EXANIMA_DIR`): the installed Walk, Run and Backpedal clips keyed every 10 frames from 10 to 80

## 6. Verification

- [x] 6.1 Light runs through `gpu_run.sh --gpu 4000` (arena, `--fixed-frames --trace`, `--cursor 480,40`): walk 1.687 cm a tick, released W walks on to segment 0 and stops 2.4 cm short of its 30.1 cm goal; standing quarter turns at 120°/s, the second within 1 s refused; W + Shift flags `0x44` (latch spent), 4.337 cm a tick; RMB far + Shift `0xc` (latch kept), the reversal at 300°/s, the free walk turn 240°/s; the run braking at the wall stops 0.5 cm from its goal; backpedal 1.476 cm a tick; walking at the wall stops 2.2 cm from its goal, no fall; combat turn 300°/s (Tab, cursor to the side); duel (arenao1 `--opponent`): the duellist approaches, stops within 1.15 m and lands six blows 2.2 s apart
- [x] 6.2 Remove probes and scratch files from the tracked tree

## 7. Documentation

- [x] 7.1 `docs/PLAYER.md` (Movement), `docs/formats/RSQ.md` (Movement decision, the trace's locomotion columns), `docs/DEBUGGING.md`, `docs/PLAN.md` (Phase 3), crate READMEs (`exa_physics`, `exa_game`, `exa_ai`)
- [x] 7.2 `openspec validate --all --strict`
- [x] 7.3 `make check` equivalent on the `play` profile: `cargo fmt`, `cargo clippy --profile play --workspace --all-targets -D warnings`, `cargo test --profile play --workspace` with `EXANIMA_DIR`
