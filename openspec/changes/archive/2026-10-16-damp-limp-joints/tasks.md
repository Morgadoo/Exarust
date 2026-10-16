## 1. Limp joints

- [x] 1.1 `exa_physics::body`: `Link::length`, the smoothed length, set by `Ragdoll::new` from each slack link's posed distance
- [x] 1.2 `Link::limp` (mutable): inside the slack L ← s·L + (1 − s)·|d| and the correction strength·(|d| − L_old)/|d|·d, skipped within 1.19e-7 cm of L; beyond the slack the hard radius with L = slack; a limp pivot at slack ≤ 1.19e-7 cm; `iterate` walks a part's links mutably
- [x] 1.3 `Ragdoll::go_limp` resets every slack link's L to its distance, at most the slack (`Link::reset_length`), with the leg limit strengths

## 2. Retention

- [x] 2.1 The default environment's 0.426 / 0.25 / 0.05 per second (`per_tick`: r^(1/83), 0 for r ≤ 0); a limp part's band by its farthest particle's displacement over the last tick, strictly over 40/83 or 20/83 cm (`limp_retention`)
- [x] 2.2 Living limbs: the fast retention × u², u = 1 for humans (`living_retention`), whatever their speed

## 3. Tests

- [x] 3.1 Synthetic: a limp link with smoothing 0.755 keeps 0.197 of a 1 cm stretch after 20 full passes ((1 − s)/(2 − s)); L starts at the posed distance; at L nothing; beyond the slack the excess and L = slack
- [x] 3.2 A pelvis and femur: a hip bent while living and gone limp at rest moves under 0.1° in 200 ticks (without the reset it springs back over 5°); `go_limp` sets L = min(|d|, slack), the slack for a link pulled past it; a limp hip swing keeps 0.80 in the first tick and 0.115 after ten, against 0.86 held by the hip alone
- [x] 3.3 Retention: 0.98977, 0.98344, 0.96455 per tick; 0.49 cm fast, 40/83 cm slow, 20/83 cm resting; a lone part keeps the resting value at 6 cm/s and the fast one at 60 cm/s limp, the fast one at 6 cm/s living
- [x] 3.4 Existing tests: the momentum test also on a damping slack link; the knee hinge test strikes the damped limp knee three times as hard (8.6° bent, 1.2° off its plane); the thin-wall test at 15 m/s with the centre in front every tick; the dropped body still settles without creeping and sleeps
- [x] 3.5 Corpus (skipped without `EXANIMA_DIR`): every shipped slack link's smoothing lies between 0.640 (humerus) and 0.914, keeping 0.08 to 0.27 of a stretch a tick

## 4. Verification

- [x] 4.1 In game through `gpu_run.sh`, with a temporary sleep probe (removed), against the C3 build; no `Validation Error` in any log:
  - R6 `--limp character`: lies curled on its side, pelvis 15 cm and head 18 cm above the floor (C3: kneeling, pelvis 45 cm); asleep at tick 769 (C3 624)
  - R5 `play exanima01`: 3 of 7 corpses asleep by 25 s and at 54 s (C3: 1 of 7 at 54 s, 5 by 141 s); the 4 awake jitter in place (pelvis within 2 cm over 130 s), fastest particle 3–4 cm/s on the floor and 21–31 cm/s on the two corpses stood on cots (S2). With the old 0.6× retention the damper sleeps 5 of 7 by 41 s: the default environment's weaker resting damping keeps the jitter above Exarust's 1 cm/s sleep rule, which S1's countdown replaces
  - R3 `--shove 60 play arena --trace`: falls at tick 305, `Get Up 2` at 518, idle at 664 (C3 293 / 516 / 662), standing 105.8 cm at the end
  - `--opponent play arena`: the duellist landed no hit in 150 s, so a probe killed it at tick 700 with the combat kill's limp velocity: the pelvis reaches the floor in 0.72 s as in C3 (not over-damped) without C3's torso rebound, but the corpse sags on (head 28 → 21 cm over 4 s, fastest particle 11–36 cm/s) where C3's was still from 4 s
- [x] 4.2 Remove probes and dumps from the tracked tree

## 5. Documentation

- [x] 5.1 `docs/formats/RSQ.md` (Physical body: the retention line, the limp damper and its resets), `docs/PLAN.md` (Phase 3), `crates/exa_physics/README.md`
- [x] 5.2 `openspec validate --all --strict`
- [x] 5.3 `make check` (fmt, clippy -D warnings, tests; `play` profile)
