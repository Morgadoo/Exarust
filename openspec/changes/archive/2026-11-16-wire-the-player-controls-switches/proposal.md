## Why

Roadmap task B4 with B1 (lane `playerctl`): the player's controls
(SG_PlayerControls). Findings F350 and F352 to F357, F359 and F360 of the
audits: the spirit form's switch, the drop scripts, the power hold and the
item cursor's re-orthonormalisation had ports with test callers only; the
drag's reach limit, the combat toggle's lift cancel, the GUI drag's end on
death and the lift's pointer warp had no counterpart; an object's keyframe
tracks never started on use.

## Evidence

REA (Exanima.exe 0.9.5), read in the instructions where constants, order or
branches matter:

- The spirit form's switch and its two callers (the spirit form's start and
  its way home): the window-holds byte, the hand's left-button keep, the
  hotkey mask 4 or 0x1f, and on the start the GUI sweep with its bar bit and
  the dialogue's end. The byte's readers: the key handler's inventory test,
  the pad routine's, the Ctrl interaction and the GUI drag's claim; the shop
  window writes it too.
- The block pairs: both clear the drag's reach-limit byte and their release
  sets it; the drag handler reads it for the 330 cm let-go and the 200 cm
  anchor clamp; the controls' frame (not the controls-off frame) runs the
  drag handler.
- The item cursor's turn re-orthonormalises the held frame (second axis
  normalised, first made orthogonal, third their cross), each normalisation
  the approximate reciprocal square root.
- The combat toggle's lift cancel restores the frames the lift saved.
- The press keeps the frozen node's matrix and a flag; the drag-end hook
  relocks a put-back item and runs the drop scripts around the old spot
  unless relocked within 5 cm² (`comiss`, unordered skips); the scripts walk
  every scene object closer than 5 cm² with an object id, and a record whose
  usable section is the reconstruction device powers it down.
- The bar's cast answers −1 after a "not now" (which holds the slot with the
  clock + 500 ms); the pad routine retries the held slot each frame, clears
  it on any answer of zero or more, and once expired asks once for the
  picked power.
- The GUI drag's end: a desk left-button release when its button word holds
  the left button; called by the death, the knock-out, the level exit and
  the came-loose callback for the player.
- The object use (double click): kind 0 starts the node's tracks and plays
  the use sound, then queues the target signal (subtype 1) or runs the
  database use; nothing detaches the node.
- The hand's idle double click runs when the controls' Ctrl interaction is
  not registered (controls off).
- The lift start with the warp flag moves the OS pointer to the node's
  screen point.
- The movement request's walk latch (set on a move's first frame, held
  after the release until a double-support key segment) gates the reach,
  with the keys' heading bit re-enabling it.
- The controls-off frame's skill-screen close is gated by a global with one
  reference in the image (a read): the branch never runs.

## What changes

- `exa_game::controls_live::spirit_controls`, `PlayerControl::set_spirit_form`
  (window byte, mask), `Hand::keeps_left`; the key handler and pad routine
  read the mask and the byte; the Ctrl interaction and the drag claim refuse.
- `PlayerControl::drag_limit`, cleared by both block pairs; `drag_anchor`
  takes the limit; the drag waits while the controls are off or blocked.
- `Frame::orthonormalized` with the engine's approximate normalisation,
  run after each item-cursor turn.
- `hotkeys::enter_combat` asks the hold frame to put a lifted item back.
- `HeldItem::from_lock`, `item_cursor::drop_scripts_at`; the device pass
  runs `player_controls::nearby_scripts` on the left lock's spot.
- `PlayerControl::power_hold`, `BarAction::Retry`; the cast's answer holds
  or lets go; the frame retries.
- `controls_live::EndGuiDrag`, flow step `EndGuiDrag`, and the loose
  weapon's call.
- `object_click`: a kind 0 use sends `StartTracks` and keeps the object;
  with the controls off the double click runs the idle hook.
- `os_pointer::WarpPointer`; the press's lift asks for it.
- The reach gate reads `PlayerBody::walking_to_cursor`.

## Out of scope

The Character and Spectate modes' camera records (the free camera relies on
the Spectate record's handler bits), the controls-off frame's placement
(the drop placement at the far limit, no reach, no marker), the drop of a
lifted item onto a GUI window, the pick on each node's meshes, the reach's
navigation-region test, the linked bodies of a lifted item.
