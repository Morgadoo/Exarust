## Why

PLAN Phase 3/4 (the player's controls and the game flow). The original's
player-control singleton has, beside the controls' off byte, a block pair
(two bytes with their own setter routines). Exarust kept only the session's
playing flag in the flow's Controls step, so a knocked-out, dead or
defeated player's controls kept running, and the arena hub's loadout form
let the player walk and toggle combat behind it.

## Evidence

REA (Exanima.exe), checked against the assembly:
- The first byte's routine pair. The block stamps the controls' idle clock,
  sets the byte, drops the mouse hook (the orbit and Cursor Look) with its
  capture state, clears the key word, sets the player behaviour's suspend
  byte, clears the body's locomotion flags, moving byte and one more body
  byte, gives the pad the menu action set and clears the drag's reach-limit
  flag. The release sets the reach limit, clears the byte, puts the mouse
  hook back, clears the suspend byte and gives the pad the combat or the
  interact set by the combat flag.
- The second byte's pair: the same without the idle stamp and without the
  body's flags.
- Readers: the controls' frame (spectate or no player, then the block byte
  runs the blocked idle turn and returns; then the off byte or the second
  byte return); the left press (with a player, the block byte skips the
  pick); the key-down handler and the combat toggle (the off byte and the
  second byte; not the block byte); the player behaviour's tick (the
  suspend byte ends it after its head).
- Writers: the knock-out (block), the death (block), the revive (release),
  the practice end menu's won-ladder branch (block, practice mode only),
  the arena match tick when the manager falls (block), the teleport's click
  (block) and arrival (release); the start of play and the Spectate start
  clear the block byte directly. The second pair: the arena loadout form's
  show (on) and close (off), the arena recruit's creator open (on) and back
  (off).

## What Changes

- exa_game: `PlayerControl::{blocked, second_block}` with their pairs and
  gates (`input_suspended`, `keys_refused`, `behaviour_suspended`); the
  flow's `Step::Block` on the knock-out, the death, the revive and the
  practice ladder's last win; the arena match tick blocks on the manager's
  fall; the start of play clears the byte; the hub's loadout form holds the
  second block while open.
- exa_game: while blocked or second-blocked the movement request, the
  behaviour's steering and inputs, the mouse capture, the attack latch, the
  held wake, casting and kinesis read nothing; the block clears the move
  request, the locomotion flags and the moving byte once; the left press
  picks nothing while blocked; the idle camera takes its blocked turn (ten
  seconds after the block) or, second-blocked, does not run; the hotkeys
  and the power tree's key are refused only by the off byte or the second
  block.

## Impact

Story and duel lockstep traces are unchanged; the knock-out trace is
unchanged too (the knocked-out body is already helpless), with the block and
release logged. Behind the hub's loadout form the player stands still and
Tab does nothing. Not ported: the pad's action-set switch and the drag's
reach-limit flag (Exarust has neither switch live), the teleport (not in
Exarust), the recruit's creator (the hub does not open it).
