## 1. Countdown

- [x] 1.1 `exa_physics::body`: one sleep countdown per body (`Ragdoll::countdown`, ticks), built at 256 (`WAKE_COUNT`)
- [x] 1.2 Before integration, a limp body's countdown follows the band of its fastest part by the last tick's motion (`speed_band`, shared with the limp retention, strict over 40/83 and 20/83 cm a tick; `count_down`): fast → 256; slow → 32 from below, −1 above 32, −2 at 32; resting −2; floor 1
- [x] 1.3 A living body (with a `Drive`) is raised to 256 every tick
- [x] 1.4 Replace the 1 cm/s for 166 ticks rule (`SLEEP_SPEED`, `SLEEP_TICKS`, `fastest`)

## 2. Sleeping

- [x] 2.1 The post-step sleeps a body at a countdown of 1 (countdown 0, `Sleep::Asleep`)
- [x] 2.2 The first sleeping tick copies the positions into the previous ones and sets `Sleep::Deep`; a deep sleeper is not touched; `Ragdoll::tick` and `step_bodies` both honour the state
- [x] 2.3 Wakes: `wake_to(n)` raises the countdown and wakes a sleeper with its stored contacts; `wake` = `wake_to(256)` (`with_drive`, `go_limp`); `touch(n)` also drops a sleeper's contacts (`shove`, so every hit, is `touch(256)`); `equalise_sleep(a, b)`, the wake of a contact between bodies, ready for C17

## 3. Tests

- [x] 3.1 Bands 0.49 cm fast, 40/83 cm slow, 20/83 cm resting; resting from 256 sleeps after 128 ticks; slow from 256 reaches 32 after 224 ticks and alternates 30, 32; floors and the fast reset
- [x] 3.2 A limp body moving at 10 cm/s sleeps in its 128th tick; at 30 cm/s it is awake after 2000 ticks with a countdown of 30–32; 60 cm/s keeps it at 256 and the 128 ticks start afresh
- [x] 3.3 A body asleep is in deep sleep after one tick, with its previous positions equal to its positions; after 500 more ticks both are bit for bit the same
- [x] 3.4 A shove wakes a deep sleeper to 256 and it moves 1.2/83 m in its first tick; a blow queued to the game's solver wakes a sleeping body
- [x] 3.5 A living body is awake at 256 after 1000 ticks; once limp it sleeps after 128 ticks at rest
- [x] 3.6 A ball resting on a floor sleeps after exactly 128 ticks and keeps its contacts; `wake_to` keeps them; `equalise_sleep` with a living body wakes it to 256 and drops them; a lower countdown is raised to the higher, never lowered
- [x] 3.7 Existing tests: the dropped humanoid still rests without creeping and sleeps (`asleep()`)
- [x] 3.8 No data format is touched; the shipped data is exercised by the in-game runs below (`play exanima01` corpses, the humanoid body)

## 4. Verification

- [x] 4.1 In game through `gpu_run.sh --gpu 4000`, with a temporary sleep probe (removed); no `Validation Error` in any log:
  - R5 `play exanima01`: all 7 corpses go limp at tick 501 and are asleep by ticks 759, 762, 763, 812, 818, 841 and 877 (3.1–4.5 s), including the two stood on cots; none woke in 75 s (a first 150 s run: 7 of 7 by tick 879). C4: 3 of 7 asleep at 25 s and at 54 s. The 33 living bodies (the player's included) stay awake at 256 throughout
  - R6 `--limp character`: limp at tick 31, asleep at tick 399 (4.4 s; C4 tick 769 under the old rule), pelvis 11 cm above the floor
  - R3 `--shove 60 play arena --trace`: falls at tick 306, `Get Up 2` at 521, idle at 667 (C4 305 / 518 / 664), standing 105.8 cm at the end; the living body stays awake at 256 while fallen and getting up
  - Forced kill (probe, `--opponent play arena`): the duellist goes limp at tick 701 with 256 and sleeps at tick 1002 (3.6 s), pelvis 13 cm above the floor; in C4 the same corpse still sagged at 11–36 cm/s after 4 s
- [x] 4.2 Remove probes and dumps from the tracked tree

## 5. Documentation

- [x] 5.1 `docs/formats/RSQ.md` (Physical body: sleep), `docs/PLAN.md` (Phase 3), `docs/PERFORMANCE.md` (physics sleeping), `crates/exa_physics/README.md`
- [x] 5.2 `openspec validate --all --strict`
- [x] 5.3 `make check` equivalent on the `play` profile: `cargo fmt`, `cargo clippy --profile play --workspace --all-targets -D warnings`, `cargo test --profile play --workspace` with `EXANIMA_DIR`
