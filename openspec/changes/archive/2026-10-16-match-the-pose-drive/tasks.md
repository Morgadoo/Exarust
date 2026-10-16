## 1. Pose drive

- [x] 1.1 `exa_physics::body`: each part keeps its own origin's weights and each particle's last displacement as integration finds it (before inertia)
- [x] 1.2 `pose_push` in cm: 1.19e-7 cm² threshold, c = o·v/(s|o|²), no push at c ≥ 1, o·(1 − c), vertical × 0.1, times G
- [x] 1.3 `drive_parts`: targets R·r + O (the target's rotation, the part's origin after integration), gains by bone (pelvis, feet, lumbar, thorax 0.03·K; head 0.01·K), `Drive::pose_strength` t²·T·E⁴·(0.5T + 0.5)·S² and `Drive::pose_gain`, gated by the has-support flag
- [x] 1.4 Remove the critical-damping PD, `DRIVE`, `SUPPORT`, `FALLEN_TONE`, the pelvis-aligned fallen targets, `Bulk` (bulk damping, net shift and turn), `Drive::before`; keep the 25 m/s guard
- [x] 1.5 `fall()` keeps the tone and stops the carry; `stand()` leaves the tone; energy recovers 0.005 a tick in the step, capped at 1
- [x] 1.6 Exarust's own controller hold: a standing body's pelvis origin toward the pose's pelvis, 0.168 × tone² × E⁴, critically damped in the carried frame, the same push on its four particles

## 2. Tests

- [x] 2.1 Gains: K = 2.7870; idle 0.0314 (head 0.0105), walking 0.0497, running 0.0836; none on limbs; E = 0.5 a sixteenth, S = 0.5 a quarter
- [x] 2.2 The push: still G·o, half closed (1 − c) = 0.5, c ≥ 1 none, c = −1 twice, across full, vertical a tenth, the threshold
- [x] 2.3 A rigid translation gives no push; a turned part is turned back about its origin (yaw by G, a tilt by less); a moving part is pushed against its motion only (c ≥ 1 none, c < −0.5 more than 1.5·G·o)
- [x] 2.4 No support, no pose drive (lying on its back, perched); a fall keeps the tone; energy capped at 1
- [x] 2.5 A standing body recovers from a 1.5 m/s chest shove (sways > 3°, balance ≥ 0.6, back within 1°, at rest); the controller holds a standing body the floor pushes 1 cm up (rises < 2 cm, rests within 1 cm)
- [x] 2.6 Existing tests: the limbs left to their muscles (turned, not moved); a carried body's contacts (three ticks); the fallen-body scenarios (falls as fast as a limp body, kicking legs within 30 cm, not flung far from the origin) pass with link muscles alone
- [x] 2.7 Corpus checks (skipped without `EXANIMA_DIR`): the shipped body's joints and legs, unchanged

## 3. Verification

- [x] 3.1 In game through `gpu_run.sh` with `--trace`: R1 `play arena` standing (balance 1, counter 80, S 1); `--autowalk` (no false falls); R3 `--shove 60` (falls, gets up with `Get Up 2`, stands at 1 / 80); R4 `--opponent` (hits land; the struck body sways, no knockdown); R5 `play exanima01` (no NPC falls); R6 `--limp character`; no validation errors
- [x] 3.2 Remove probes and dumps from the tracked tree

## 4. Documentation

- [x] 4.1 `docs/formats/RSQ.md` (Physical body: the pose drive, the controller hold, falls), `docs/PLAYER.md`, `docs/PLAN.md` (Phase 3), `crates/exa_physics/README.md`, `crates/exa_game/README.md`
- [x] 4.2 `openspec validate --all --strict`
- [x] 4.3 `make check` (fmt, clippy -D warnings, tests; `play` profile)
