## Why

Gap lane `gamemain` (SG_GameMain unit routines, TEndSessionMenu,
TResetToPortalDlg). Two reachable routines had a port without a live
caller: the story continue's exit-trigger nudge (SG_GameMain, 719 bytes)
and the "Reset to portal" dialog (its control table, the question its
opener asks and the reset its Yes runs), which the saves' screen did not
build.

## What the original does

REA and the assembly of the story continue, the nudge, the dialog's
constructor, its control table, its Yes and No callbacks, the question's
check and the save manager's reset:

- **Story continue** loads the slot, places the player and nudges it out
  of the first trigger in its sector that holds it (within its radius plus
  50, within 50 squared of its box, the object's exit bit): along the axis
  with the smaller clearance, past the box's side by 75, then stood on the
  ground; the camera frames the moved player.
- **The question** runs after each select of the character screen and
  when it opens in continue mode: story mode only, a save whose player
  data's third flags byte is 0 (every 0.9.5 writer sets it), with an
  `exanima06` section, or with `exanima04` and a world object database
  magic other than 0xDBAC0D06.
- **The dialog** (`Reset to portal`, 392 x 112) holds three records: the
  question label, `Yes` (the reset) and `No` (close with result 0).
- **Yes** closes and deletes the save manager's open package (the file the
  select read), then drops the slot from the list and clears the current
  slot unless the slot's checkpoint (`.rcp`) still opens; the creator
  selects the same index again behind a fade and the dialog closes.

## What Changes

- `exa_game::flow`: `Step::ExitNudge` after the story continue's
  placement, applied as `level_change::NudgeOutOfExits`;
  `level_change::nudge_out_of_exits` runs `flow::exit_nudge` over the
  level's exit zones; the binary's placement moves the player before the
  floor cast.
- `exa_game::slots`: `offers_portal_reset` (on
  `save_package::story_start_incompatible`) and
  `SlotList::reset_to_portal`.
- `exa_game::save_screen`: the reset box raised after a select or at open,
  drawn and hit like the delete box, Yes and No; `--portal-reset yes|no`
  answers it (debug).
- `exa_ui::dialogs`: `RESET_RECORDS`, `RESET_QUESTION`, `reset_box`.

## Capabilities

### Modified Capabilities

- `game-flow`: the continue's exit nudge.
- `save-games`: the reset-to-portal question and its reset.

## Impact

- Code: `exa_game::{flow, level_change, save_live, save_screen, slots}`,
  `exa_ui::dialogs`, `exarust::{main, viewer}`.
- No simulation change: lockstep exanima01 traces identical to round 200.
