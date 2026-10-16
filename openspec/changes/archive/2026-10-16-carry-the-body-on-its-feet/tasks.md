## 1. Carry and lift

- [x] 1.1 `exa_physics::body` (`body_lift.rs`): gravity per tick, the support shares (pelvis and thighs 1, lower legs 0.5, feet 0.25) and the carry share (tone × share, or tone × energy; none at tone ≤ 0.5)
- [x] 1.2 `lift_factor` in cm: standing 0.075(h + 15) ≤ 1.5 (negative none, crouched past 0.9 down to −10), moving (0.4·run + 0.2)(h + 5) ≤ 2 (negative none), damping (1 − |f|/(|f| + 2))·E²·v
- [x] 1.3 `target_height` in cm: per leg ground + crouch·60 + (1 − crouch)·stance, the lowest foot, the other blended by half under the spread test (40/60, 0.05 a cm), standing capped at ground + sole + leg length and raised 5; no ground −100000
- [x] 1.4 `Ragdoll::lift` at the end of `aim`/`hold`: leg ground offsets (sole; moving, the ankle over the walk's lowest), stance from the targets, the target, then with support f × 2(t − 0.5) and carry = g(1 − f·S); `Drive::carry`, `lift_target`, `lift`
- [x] 1.5 `integrate`: each particle g(1 − a) + carry·a over half its tone, else g; the `(1 − tone·energy)` gravity removed
- [x] 1.6 The capsule carries only x, z and yaw; the controller hold across the ground only
- [x] 1.7 `Leg::offset` read by the grounded test; `Bearing::gait_floor` from the moveset's Walk first pose in `exa_game::limp` while moving
- [x] 1.8 The body trace's `target_pelvis_height`

## 2. Tests

- [x] 2.1 The lift's law: f = 1 at 1.7 cm over the target, 1.5 cap, none 15 cm over; moving 1 at the target walking, 3.3 cm over running, cap 2; crouched −10; damping 2/3·E² at f = 1, (1 − 1.5/3.5) at the cap
- [x] 2.2 Shares; the target: idle human 99.83 cm, a 30 cm step +30, one foot up capped (moving the mean), blend 0.5 / 0.125 / none, a high floor, no ground −100000
- [x] 2.3 The pelvis settles 1.7 cm over its target (f ≈ 1), set down 1 cm into the floor rises < 2 cm
- [x] 2.4 The target follows a 30 cm step down (target −30 cm, pelvis −30 cm, balance kept, landing dip < 2 cm)
- [x] 2.5 A dazed body sags: hovering at E 1 / 0.75 / 0.5 the pelvis settles 0.28 / −1.21 / −3.05 cm off its target (f = Σm/Σam)
- [x] 2.6 No support: carry = g, every particle falls at g; a fallen body is not lifted; tone 0.75 halves the lift, 0.5 none
- [x] 2.7 The hold moves the pelvis across the ground only; the carried body's contacts move across the ground only
- [x] 2.8 Existing tests ported (a standing humanoid on a floor for the turning and chest-shove tests; legless bodies lie on a floor)
- [x] 2.9 Corpus checks (skipped without `EXANIMA_DIR`): the shipped body's joints and legs, unchanged

## 3. Verification

- [x] 3.1 In game through `gpu_run.sh` with `--trace`: R1 `play arena` (standing height, 1/80/1); `--autowalk` (no falls, pelvis bob); R3 `--shove 60` (falls, gets up, no hop); R4 `--opponent` (hits); R5 `play exanima01` (no NPC falls; stairs); R6 `--limp character`; no validation errors
- [x] 3.2 Remove probes and dumps from the tracked tree

## 4. Documentation

- [x] 4.1 `docs/formats/RSQ.md` (Physical body: the carry, the lift, the target, the capsule's carry, the trace column), `docs/PLAYER.md`, `docs/PLAN.md` (Phase 3), `crates/exa_physics/README.md`, `crates/exa_game/README.md`
- [x] 4.2 `openspec validate --all --strict`
- [x] 4.3 `make check` (fmt, clippy -D warnings, tests; `play` profile)
