## Why

PLAN Phase 3/4 (the player's controls and the game flow). The skill screen
went live with no way to take the player's controls away: with it open, the
movement keys walked the player, the clicks reached the world and the hotkeys
ran. The original turns the controls off whenever such a form opens and back
on when it closes. The story start's companion (Derrin) faced -90° where the
original faces him -90 radians.

## Evidence

REA (Exanima.exe), checked against the assembly:
- The controls' switch takes the controls' off byte (set by the
  constructor). Off: the mouse hook for the orbit and Cursor Look is dropped
  with its capture state, the key word, the behaviour's move flag and the
  locomotion flags are cleared, the click, Ctrl and right-button handlers
  leave the view's cursor actions, the hand's three per-frame bytes are
  cleared, the controls' frame task is swapped for the hand's own, the right
  button goes to the GUI and the pad takes the menu action set. On: all of
  it comes back and the combat toggle is re-applied.
- The key-down handler and the frame task return at once while the byte is
  set.
- Its ten callers: the Character mode's start (on) and end (off), the
  Spectate mode's start (on) and end (off), the skill unit's open (off,
  before the screen is built) and close (on), the power tree's open (off)
  and close (on), the arena arsenal's open (off) and close (on). The
  inventory, the character view and the menus do not call it.
- The story start, for the first background, spawns `xac_derrin` through
  the character spawn hook at (5808, 0, -4075) with the record's facing word
  -90 passed unconverted; the hook takes radians (the hub's placements
  multiply their degrees by pi/180 first).

## What Changes

- exa_game: `PlayerControl::off` and `set_off`, the live switch
  (`controls_live`), wired to every opener and closer; the key handler, the
  pad routine, the movement request, the behaviour's inputs, the mouse
  capture, the idle camera, the casting and kinesis inputs read nothing
  while the controls are off; the skill screen and the power tree count as
  the pointer over the GUI.
- exa_game: the pad's Select outside a dialogue confirms the end menu's row
  while it shows (the controls' pad routine's end-menu call).
- exarust: the story companion's yaw is the start's facing word in radians.

## Impact

With the skill screen, the power tree or the arsenal open the player stands
still and the world takes no click. Story and duel traces are unchanged
tick for tick. Derrin, at a new story, faces -90 rad (about -116.6°).
