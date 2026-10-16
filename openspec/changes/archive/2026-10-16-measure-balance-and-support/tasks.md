## 1. Balance and support

- [x] 1.1 `exa_physics::body`: `PartSetup::pins`; the body keeps its chest pins (weights on the thorax), its legs (lower leg, contact at the foot part's origin, leg length, ground, grounded) and the sole height; `Part::foot` becomes the ankle; `Drive` keeps the `Balance`
- [x] 1.2 `body_balance.rs`: chest point and its two smoothed velocities (0.4 / 0.1, one tick late); support point on the ankles' segment, halfway to their centre standing; landing (5, −0.05), moving lead (3 × fast), crouch (−10 cm); score u_y⁸ / u_y⁴ + 0.05 × bonus; the vertical cut and counter 100; the counter (round half to even, −5 floor 80 standing still, −10 floor 0) and balance (q at 100, recovery (5 − v)·0.01, 0.05 at rest); the get-up branch; all in cm through one helper
- [x] 1.3 The feet: a Height Query per leg (5 cm along the lower leg's forward axis, 70 cm above the contact with a 10 cm sphere, or at it with a 5 cm one while down or getting up, 2000 cm, a miss at −10000 cm); no ground above half a leg under the pelvis (except getting up); grounded within 20 / 30 cm of ground + sole; has-support, landing on regaining it, airborne counter; balance 0 standing without support (unless the controller carries the body); S = grounded × balance; getting up within 30 cm of the ground under the pelvis: support at half balance
- [x] 1.4 `contact.rs`: `Level::height_below`, the column's upward front faces highest first, an analytic drop onto face, edges and corners, a 2 m column before the whole depth
- [x] 1.5 `body_muscle.rs`: `Bearing` gets `getting_up`, `crouch` and `carried` and loses `grounded`; `RaceMuscles::balance_bonus`; `tune_muscles` reads the measured S (bodies without two legs: 1 standing, 0 fallen) and stiffens the arms below balance 0.6
- [x] 1.6 `exa_game::limp`: the character tick measures balance against the physics world before aiming; the bearing (getting up, carried); the pins from the skeleton; `exarust` passes the race's balance bonus
- [x] 1.7 `exa_game::balance`: the fall trigger in the fixed step before the character ticks, from last tick's balance, Fall Forward or Backward by the lean; the get-up's balance ramp moves into the body; the get-up's controller settles on the ground; the `Balance` component and Exarust's rule go
- [x] 1.8 The trace writes S, balance and the counter from the body

## 2. Tests

- [x] 2.1 Synthetic, pinned to the original's numbers: chest = mean of the pins; legs on their ankles (leg length, sole 8 cm); an upright body never falls (1 / 80 / 1 for 1000 ticks); balance 0 without ground (airborne counter, a miss at −100 m), kept moving or carried; landing 5 → 4.95; smoothing 0.4 / 0.1 a tick late; the lean table (6–60 cm at 125 cm), u_y⁴ moving, 44 cm at 130 cm = 0.698, under 0.6 past 52.2 cm; rounding half to even (0.95 → 0, 0.75 → 2, 0.55 → 4); counter rules and recovery; the floor of 80 (25 against 5 ticks); the vertical cut (sinking 5 cm a tick: counter 100 on the tenth tick); grounded within 20 / 30 cm; ground out of reach under one foot; down or getting up, the query from the ankle with a 5 cm sphere (the floor under a 40 cm shelf, past a 5 cm step's edge 7 cm away); getting up near the ground (S = 0.5·balance)
- [x] 2.2 The Height Query: floor, a step's edge at 28.66 cm, a downward face passed, starting against a face, behind it, past the depth; the analytic drop against the exact distance on 4000 random cases
- [x] 2.3 Existing tests: the leg limit without ground (measured, not a flag); the leg relaxation of a straight leg on its ankle (0.2); the trace's balance columns; Fall Forward / Backward by the lean
- [x] 2.4 Corpus (skipped without `EXANIMA_DIR`): the shipped body's legs stand on the foot origins (1.24, −41.87, −2.32) cm, legs 85.15 cm; chest pins (0, 9.65, 1.25) cm

## 3. Verification

- [x] 3.1 In game through `gpu_run.sh`, with a temporary probe (removed): R1 `play arena` standing (every tick balance 1, counter 80, S 1); `--autowalk` walking (balance 1, no falls); R3 `--shove 60` (falls backward, gets up with `Get Up 2`); R4 `--opponent` (hits land, no false falls, the duellist's balance stays 1); R5 `play exanima01` (no NPC falls; living NPCs at 1 / 80 / 1); a standing drop from 3 m (airborne: balance 0, S 0, falls and gets up); the Height Query's cost (about 15 µs per body tick); no validation errors
- [x] 3.2 Remove probes and dumps from the tracked tree

## 4. Documentation

- [x] 4.1 `docs/formats/RSQ.md` (Physical body: balance, the Height Query, support, the ankle contact, the fall trigger, trace columns), `docs/PLAYER.md`, `docs/PLAN.md` (Phase 3), `crates/exa_physics/README.md`, `crates/exa_game/README.md`
- [x] 4.2 `openspec validate --all --strict`
- [x] 4.3 `make check` (fmt, clippy -D warnings, tests; `play` profile)
