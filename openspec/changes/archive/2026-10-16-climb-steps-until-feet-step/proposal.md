## Why

C8 (`carry-the-body-on-its-feet`) holds the pelvis at the original's
target height over the lowest foot's ground. The original climbs stairs on
its feet: a walking foot swings over the step ahead (swing flags, a ground
probe ahead, C19), the trailing foot closes up, and the legs bend onto the
steps (leg IK, C13). Exarust has none of that yet, and its capsule steps up
before the feet: the target stays a step under the capsule, the feet catch
the risers and the body falls. In exanima01's stair room
(`--autowalk --at 7.5,-0.95,64.4`) a walker fell 5 and 4 times in two
runs of about 50 s, and from `--at 7.6,-1.3,66.2` once; C7, which carried
the body on the capsule, climbed with none.

Phase 3 (Bodies), milestone M3 "standing man": keeps stairs climbable
between C8 and C13/C19. It is Exarust's own stopgap, not the original's.

## What Changes

- While a living body that has not fallen has its character controller
  on the ground (`Bearing::carried`), standing or moving, the lift reads each foot's
  ground no lower than the controller's floor less 6 cm. Everything else
  in the lift and the target pelvis height stays the original's.
- Over level ground the capsule floats 2 cm walking and up to 5.05 cm
  standing over the ground the feet's Height Query finds, so nothing
  changes there. On a step the target follows the capsule up, 6 cm under
  its floor; a body stopped on the stairs stays up there instead of
  sinking onto its trailing foot.
- A new module `exa_physics::body` `climb` (`body_climb.rs`) holds the rule
  and is labelled as Exarust's own; C13 and C19 remove it.

Evidence:
- Exarust runs through `gpu_run.sh --gpu 4000` with `--trace`
  (`target_pelvis_height`, `pelvis_height`, `frame_height`): the C8
  build against this one at both exanima01 stair starts, `--autowalk play
  arena`, `--shove 60 play arena`, `--autowalk play exanima01`,
  `character`.
- The original's climbing rules this stands in for are in
  `context/re/body-feet.md` and `body-drive.md` §14 (C8's port notes);
  nothing new is read from `Exanima.exe`.

Out of scope:
- the original's walking feet (swing flags, the probe ahead, the closing
  step, the gait-smoothed ground: C19) and leg IK (C13), which replace
  this stopgap;
- descending stairs (the capsule then stands under the feet, so the rule
  does nothing);
- the spawn fall at `--at 7.5,-0.95,64.4` (the capsule is still in the air
  at the body's first tick, its feet 20 cm over the ground; C7 and C8 have
  it too);
- the C8 probe's jumps when a foot's probe finds a higher step while the
  capsule is held up at a wall.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `physical-body`: a living body climbs and stands on the steps its
  grounded controller climbs (Exarust's own, until the feet step).

## Impact

- `exa_physics` (`body.rs`: the `climb` module; new `body_climb.rs` and
  `body_climb_test.rs`; `body_lift.rs`: `Ragdoll::lift` reads the grounds
  through it; `body_muscle.rs`: `Bearing::carried` documented)
- `docs/formats/RSQ.md` (Physical body), `docs/PLAYER.md`, `docs/PLAN.md`,
  `crates/exa_physics/README.md`
