## Why

PLAN Phase 2 (engine core and camera). The component gap lane for the
navigation unit and the system unit listed both components routine by
routine. The navigation unit's controller constructor, preset and
placement were tested models with no caller: the chase camera kept its
own rounded spring constants (0.0718, 0.616) and turned to a scripted shot
with `atan2`, ignoring the shot's distance. The system unit's
floating-point mode routine, which every thread of the original runs
first, had no counterpart: Exarust's threads ran with denormals.

## Evidence

REA (Exanima.exe), read against the assembly:
- The camera switch builds the game's camera controller with one of two
  preset records the player controls hand it. Both give view 250, the
  default retention, size 6000, rate 0.01, first distance 1300, distances
  800 to 2000 and looked-at heights 0 to 200; they differ only in handler
  bits (the free press drag). The controller's tick multiplies the yaw
  spring by the preset's view over 59² and its retention's 59th root:
  250/3481 = 0.07181844 and 0.61624608, which Exarust had rounded.
- The game's camera shots (session start, next fight, story start and
  continue, arena shots) set the camera's eye and target, then call the
  placement: the yaw and its shown copy become the engine's planar yaw
  (the fast arc cosine) of the backward vector; a fast shot passes 1300 as
  the distance, which becomes the shown zoom and, raised to the smallest
  and on the stepped camera lowered to the largest distance, the zoom
  target. A slow shot passes the controller's own last distance.
- The system unit's mode routine ORs `0x8FC0` into MXCSR: flush-to-zero,
  denormals-are-zero and the exception masks; rounding stays to nearest.
  The program start calls it on the main thread, the system unit's thread
  entry and the operating-system unit's thread entry call it first. The
  simulation thread is the latter's, created at priority 15 (time
  critical).
- The character view's camera (inventory, creator, trade, arena roster)
  uses a third preset (distance 410, pitch 0.15, sensitivity 2); it stays
  out of this change.

## What Changes

- `exa_game::nav_camera::GAME_CAMERA_PRESET` (and the second record's
  flags); the chase camera builds its controller from it with
  `NavController::new` and takes the yaw spring and zoom ease from the
  applied preset; the zoom bounds and start are the preset's distances.
- `ChaseCamera::face_shot` places through `nav_camera::place` (engine yaw)
  and takes a fast shot's distance as the zoom; `flow::CameraFrame::speed`
  is renamed `distance` (it is the placement's distance, not a speed).
- `exa_physics::float_mode`: `enter` sets the engine's MXCSR bits; the
  main thread, both `exa-sim` thread kinds and every Bevy task-pool thread
  enter it.
- Thread priority is not raised (documented in `sim_app`).

## Impact

Scripted `--fixed-frames --trace-all` runs of exanima01, exanima06 and the
arena duel are identical to main tick for tick (no denormal reached a
traced value; no shot runs in those scripts). A camera shot's yaw moves by
the fast arc cosine's error (about 5e-4 rad for the story shot) and a
fast shot resets a wheel-changed zoom to 1300. The yaw spring is 0.03 %
stiffer and retains 0.04 % more.
