## Why

A standing body in Exarust is still placed by its capsule: its targets
are the capsule's transform times the clip pose, and the capsule's hold
pulls the pelvis back to where the capsule carries the pose's pelvis. A
body pushed off its spot is pulled back to the capsule, and a get-up moves
the capsule under the body in one go (the camera jumps with it). The
original has no capsule: it puts the pose on a character frame whose
origin follows the pelvis by its own rules, frozen while the body stands
steady, so a steady body keeps its stance spot and an unsteady one stays
where it is pushed.

Phase 3 (Bodies), milestone M3 "standing man". This is change C9 of the
body-controller plan; it depends on C6 and C7 (done) and on C8 (done).

## What Changes

- The character frame: a living body's targets are its clip pose on a
  frame that turns with the body (the body root's turn); its origin, the
  character's place, is the pelvis origin less the pose's root offset,
  across the ground, at the pelvis's height. Each character tick it
  follows the pelvis by the original's rules:
  - moving: onto the pelvis origin itself, every tick;
  - without support: to the pelvis less the root offset, every tick;
  - standing on its feet: past 5 cm while support is under 0.95 (compared
    as a double), not at all from 0.95, its height kept at the pelvis's;
  - down (off balance, balance under 0.3): past 5 cm, in three axes;
  - getting up: past 10 cm while the get-up clip is in its first quarter,
    then not at all.
  It is placed at once the first time, and carried with the controller's
  carry.
- The capsule's hold (Exarust's own until the support pins, C14) pulls the
  pelvis toward where the frame puts the pose's pelvis; while the body
  moves only its damping acts.
- The controller follows the frame (Exarust's own): each fixed step moves
  its feet a tenth of the way to where the frame's origin puts the body
  root, at most 2 cm, through the controller, before its own walk; the
  body's carry leaves that move out. The camera, which follows the
  controller, so follows the body without jumps.
- `get_up` turns the body but no longer moves the controller: the frame's
  re-anchoring places the clip on the body.
- The body trace adds `frame_x`, `frame_z`, `root_x`, `root_z`.

Evidence:
- Reverse engineering of `Exanima.exe` 0.9.5 with REA and `exe.asm`: the
  anchor routine (the stepping branch, the root offset subtraction, the
  three-axis squared distance against the tolerance), its callers in the
  body tick's standing branch (down 5 cm; stepping or no support 0;
  supported: height every tick, 5 cm under support 0.95), the locomotion
  branch's direct write of the pelvis, the get-up start's 10 cm and its
  first-quarter test, the re-pick under 0.4; constants read with
  `read_bytes` (5.0, 10.0, 0.0, 0.95 double, 0.25, 0.4 double)
  (`context/re/body-drive.md` §6 and the new §15, `body-misc.md` §2.7,
  `body-feet.md` §9.5).
- Exarust runs through `gpu_run.sh` with `--trace`: `play arena`
  standing, `--shove 60`, `--autowalk`, `play exanima01`.

Out of scope:
- the support pins (C14) and the foot targets (C12), which will read the
  frame; the jolt that moves the stance (C15); the collapse's every-tick
  follow (C23); stepping (C15, C16);
- the clip blends and the get-up's lead-in (C10), so a get-up re-anchors
  with its own first key's root offset instead of the last sampled one;
- the original's falls, get-up pick and heading turn (C11), so Exarust's
  lying clip (Unconcious 1, its root 57 cm off) still sets where the
  controller waits while the body lies;
- the camera following the pelvis as the original's does (Q12).

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `physical-body`: the character frame; the controller following it; the
  living body's pose on the frame and the hold toward it; the get-up's
  placement by the frame; the clip's targets on the frame; the trace's
  frame and root columns.

## Impact

- `exa_physics` (`body.rs`: `Drive::anchor`/`frame`, `aim`/`hold`,
  `carry`, `followed`, `hold_on_frame`; new `body_frame.rs` and
  `body_frame_test.rs`; `body_muscle.rs`: `Bearing::rising`; `lib.rs`:
  `CharacterBody::follow`/`followed`, `FOLLOW_SHARE`, `FOLLOW_MOST`; tests
  in `body_drive_test.rs`)
- `exa_game` (`limp.rs`: the get-up's first quarter, the follow; `balance.rs`:
  `get_up`; `trace.rs`: the frame and root columns)
- `docs/formats/RSQ.md` (Physical body), `docs/PLAYER.md`, `docs/PLAN.md`,
  `crates/exa_physics/README.md`, `crates/exa_game/README.md`
