## Why

Roadmap B4, TPlayerBehaviour (lane playerbeh), with the SG_PlayerControls
leftovers of playerctl2. The behaviour's pad aim eased a world point (the
original eases a screen cursor and interpolates it per frame through the
behaviour's slots 19 and 20, both uncalled); the screen cursor's setter
had no caller; the mouse capture ended on the pad's camera stick, which
the original never does, and ignored the pad's look buttons; the
request's two gates (the kinesis pick's drag, action 45) were not read;
the aim point's raise was capped at the ground and its bone lock used
guessed points; slot 23 and the death hook were not called.

## Evidence

REA (Exanima.exe), read in the assembly (objdump alongside):
- The behaviour tick (4,192 B): after the contacts, a shock's stagger
  (state flag 0x100) or the behaviour's disable byte returns before the
  practice, the key poll, the cast asks and the ladder; the right
  button's branch keeps the mode while the body plays action 45; the
  ladder's dash branch is the moving byte with a speed over 2; the camera
  follow waits for Cursor Look, the capture and the combat Interact byte.
- The aim read (1,515 B): its first branch is the capture (the global
  the pad reader also tests is the capture's active flag); the pad's
  screen cursor eases by `(0.4·AimSpeed + 0.1)²` (double) toward the
  projected stick point, starts at the pointer through the setter, writes
  the GUI pointer rounded (CVTSS2SI), and is let go with a pointer warp
  when the stick falls under the single epsilon; during the capture a
  moving pair (squared, double, over 0.01) latches, Cursor Look's kind
  turns the follower to `target − 0.04·x` (double) and aligns the camera,
  the orbit's kind calls the look input with the pair. The cursor read
  takes the aim point (slot 32) only for action 12 (Overhead Swing), else
  the plane pick from the GUI pointer.
- Slot 19 (`prev = cur; cur = target`) is called by the body's post-step,
  slot 20 (`shown = prev + (cur − prev)·t`) by the render interpolation;
  the GUI's world cursor draw reads the drawn point, rounded, while the
  pad drives it.
- The aim point (1,478 B): root the target's pelvis part, points its head
  part (moved along its up axis) and the two foot-contact lower legs'
  nodes; the raise caps at the player's pelvis height plus 200 cm.
- The capture (1,310 B): the pad's 0x100000 and 0x200000 bits count as
  the look and orbit keys; with the screen cursor driven or the pad
  latch set, a held capture saves the view and answers 1; the release
  calls the setter; the frame skips the desk's mouse-move update when
  the routine answers 1.
- The request (2,215 B): action 0x2d (Head Pain) and the hand state
  0x40000 (set only by the kinesis pick hook) clear the request after S's
  flags; a NaN cursor distance takes the walk.
- Slot 23 is called by the ground, frame and living placements; slot 25
  by the body's death, releasing the desk's capture.

## What Changes

- New `exa_game::pad_cursor`: the screen cursor, its post-step advance
  and frame draw, the view the frame hands the step; `AimCursor` and the
  world-point ease are gone.
- `player::cursor_look`: the pad buttons, the pad hold, the release's
  setter write, the answer (`CaptureAnswer`); `gui_screens` drops the
  pointer move on it. The gamepad reader's raw-aim flag is the capture.
- `player::CursorPlane` beside the aim point; the cursor read's choice.
- The request's drop (`Steering::dropped`, `hand::Holding::Callback`);
  the ladder's dash and Head Pain; the halt on the stagger; the camera
  follow's gates; `PlayerBody::placed` from the placements; the clinch's
  stance; `release_on_death`.
- The world cursor's draw position (`exa_ui::WorldCursor::at`).

## Impact

Lockstep: the pad's aim and the overhead-only aim point change the
player's cursor direction when a target is locked or a surface raised the
point (scenes with the player aiming in combat); the dash input changes
the ladder's button word; nothing else in the simulation changes.
