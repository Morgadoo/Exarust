## Why

Roadmap task B4 with B1 (lane `cammodes`): the camera control, the game
modes and the 3D view. Findings F361 and F362 of the audits: the player's
chase camera was built from the Spectate mode's record whatever the mode,
the Character mode's record was test-only, and the game mode table was
registered but nothing live read it. The viewport's view-to-screen routine
had a port with test callers only, and the final pass's sharpen stayed at
the 4× level's strength whatever the settings said.

## Evidence

REA (Exanima.exe 0.9.5), read in the instructions where constants, order or
branches matter:

- The two mode starts hand the camera switch their records: the Character
  start the record with flags 0x200300 (its first and smallest distance
  written to 800 and 600 when the Steam client reports a Deck), the
  Spectate start the record with flags 0x220b01, or'd with 0x400000 for the
  call and cleared after it. Both records hold view 250, size 6000, rate
  0.01, distances 1300 in 800 to 2000 and heights 0 to 200.
- The camera switch runs the leave routine of the mode the scene's word
  holds (a word of 3 or more, the table's record at the word less three),
  stores the new word, then retargets or creates the controller and
  applies the record. The other callers switch to word 0 with no record.
- The preset routine copies the record's flags into the controller's flag
  word first. The only other write to that word in the unit is the scan's
  0x800000; no record the game hands over carries 0x1000, so the tick's
  scene lift and its contact callback never run.
- The tick reads the pan keys and adds the pad's move vector only under
  0x800; it flattens under 0x20000. Under 0x400000 it holds the free point
  in the scanned box and copies the follow point.
- The room walk compares the scene's mode word with the Character mode's id
  each frame.
- The view-to-screen routine divides by the depth with the projection's x
  and y scales over the view size; any point not strictly in front of the
  eye, and a NaN depth, takes the edge branch, where 0 is taken only for a
  coordinate strictly below 0.
- The level setter stores 1.0 × (0.075·level + 0.4) (0 at level 0) on the
  view; the post multiplies it by 1.5.

## What changes

- `nav_camera::CHARACTER_CAMERA_RECORD` and `SPECTATE_CAMERA_RECORD` (with
  `SPECTATE_SCAN`, `character_camera_record` and its Deck distances) replace
  the single preset.
- `GameModes` holds the scene's mode word; `GameModes::switch` and
  `enter_mode` are the camera switch's mode half, run by the start of play,
  the end of play in Character mode, `--spectate` and the spectated arena
  match. `ChaseCamera::take_record` and `take_mode_records` are its camera
  half; the wheel and the pad zoom take the held record's stops.
- The room walk and the controls' end of play read the mode word.
- The free branch adds the pad's move stick under the record's 0x800.
- `player::window_point` projects through `exa_ui::viewport::view_to_screen`
  (now with the routine's NaN branches); the hand's lift warp uses it.
- `exa_render::post::PostSharpen` follows the supersample level.

## Impact

- Visible: the spectating camera's zoom starts again from 1300 when the
  session starts; a point behind the eye reaches the window's edge; the
  lift warp lands on the item with supersampling on; the sharpen follows
  the settings (none with supersampling off).
- Owed grid scenes (GPU on hold): exploration (`play` in a story level:
  the camera holds 0x200300, zoom and pitch as before), combat mode (Tab,
  the aim lock's screen points), the inventory (a lifted item's warp), the
  arena (a spectated match: the free camera pans with W/A/S/D), the free
  camera (`--spectate`: the zoom restarts at 1300, W/A/S/D pan).
- Lockstep: the Character start does not move a new camera, so story and
  arena runs keep their traces; window points now come from the
  original's arithmetic, so an aim-lock point can differ in its last bits.
