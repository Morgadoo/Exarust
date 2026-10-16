## 1. Link muscles

- [x] 1.1 `exa_physics::body`: `Part` keeps its origin, foot contact, link muscle factor, saturation, relaxation and cap; `Link` keeps its anchor (weights on the parent), the anchor last sampled and the tick-start history; `PartSetup::foot` carries the contact point
- [x] 1.2 `body_muscle.rs`: `muscle_pull` in cm (damping D·k/(0.5|e|² + 1.4), saturation e/(1 + c|e|), the 1.19e-7 cm² floor), `Link::muscle` (relaxed anchor, ×0.6 on bones under 8, σ = strength × factor × √E·T, the excess beyond the slack, the 80 cm cap), `Drive::muscle_gains` (√E·T, (E − 0.5)², k = 50 or 40/stance)
- [x] 1.3 `iterate` uses the muscle for every living slack link with an anchor; `step` records the history from the tick's start particles after integration
- [x] 1.4 `tune_muscles` in the character tick (`aim`, `hold`): support, stance factor, thigh factors and leg limit blend, arm stiffness, factors and caps, anchors from the relative animated turns at the part origins, relaxation for every part, a supported leg's relaxation from its reach
- [x] 1.5 Starting factors: 0.5, a foot half its lower leg's, the head a quarter of its thorax's
- [x] 1.6 `drive_parts` pulls only the pelvis, feet, lumbar, thorax and head; `HAND_DRIVE` removed
- [x] 1.7 `Bearing` and `RaceMuscles` (`Physical::race`, `Ragdoll::with_race`, `set_bearing`); `exa_game::limp` sets each body's bearing from its controller, the player's run blend and turn, its clip and `Fallen`; `exarust` fills the race from `races.rdb`
- [x] 1.8 The trace writes the stance factor, the tone target and support
- [x] 1.9 `exa_ai`: the duellist's strike range 1.2 m (link-muscle arms reach less far)

## 2. Tests

- [x] 2.1 Synthetic, pinned to the original's numbers: idle damping gain 14.3 (8.6 on the legs), walking 8.9, a skeleton's k = 53.3; strength ∝ √E and no damping at E ≤ 0.5; the pull (10 cm → 5 cm, 1.703 cm with damping); stance factor 0.5 → 0.7 in 20 ticks → 1 running; arm stiffness 0.4 (skeleton 0.3), 0.2, 0.5, the turn floor, caps 0.36/0.52/0.28; relaxation rules and the leg's target; anchors at the relative animated joint; λ to 0.36, unchanged by fast motion, 0.28 at 0.1 cm; thigh 0.25/0.2 and leg limit 0.23 → 0.5 → 0.4 → 0; part factors 0.5/0.125/0.25 and arm σ 0.25/0.175/0.3125
- [x] 2.2 The elbow follows a 40° swing within 5° 20 ticks after it ends, without overshoot; one muscle pass keeps the momentum; the pose drive leaves arms and legs alone
- [x] 2.3 The synthetic humanoid's joints get a link muscle (12 cm along, 12 cm slack); the kicking fallen body kicks its legs (more than 20°) and stays within 30 cm; the limp sleeping body keeps hinges only (`hinged`)
- [x] 2.4 Corpus (skipped without `EXANIMA_DIR`): the shipped body built through the game's `part_setup` with its foot contacts, as before

## 3. Verification

- [x] 3.1 In game through `gpu_run.sh`, with a temporary joint-error probe (removed): `character` Idle (joints ≤ 1°, feet 3–5° on the cobbles), Walk (≤ 11°), Run (≤ 19°), `One Handed/Attack Mid` with a sword (≤ 34° at 9.6 m/s, no flailing); R1 `play arena` 62 s standing (joints ≤ 0.1°, pelvis 100.7 cm above the feet, S = 1, s = T = 0.5); walking with `--autowalk` (s 0.7, arms 0.2, joints ≤ 9°); R3 `--shove 60` (falls at tick 298, lies at 354, gets up with `Get Up 2` at 504, idle at 650); R4 `--opponent` (10 hits of 17 swings, the player killed; 0 of 23 at 1.5 m; 11 of 41 before the change); R5 `play exanima01` (39 characters, living ones ≤ 0.1°, one foot 7.5°); R6 `--limp character` (kneels, forehead down, as before); no validation errors
- [x] 3.2 Remove probes and dumps from the tracked tree

## 4. Documentation

- [x] 4.1 `docs/formats/RSQ.md` (Physical body: link muscles, gains, arm stiffness, relaxation, support, the pose drive's parts, trace columns), `docs/formats/OTHER.md` (Races: arm relaxation), `docs/PLAYER.md`, `docs/PLAN.md` (Phase 3), `crates/exa_physics/README.md`
- [x] 4.2 `openspec validate --all --strict`
- [x] 4.3 `make check` (fmt, clippy -D warnings, tests; `play` profile)
