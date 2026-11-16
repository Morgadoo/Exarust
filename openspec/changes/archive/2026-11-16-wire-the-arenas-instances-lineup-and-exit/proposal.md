## Why

Roadmap tasks B4 and B1 (docs/roadmap/phase-b-close.md), the `arena2` lane:
the arena's open rows after rounds 214, 230 and 240. The season's rewards
and the week's prize reached the arsenal as object-database ids, not as
fresh item instances; the arena load never ran the arsenal's id pass; the
board had no Choose button for an event's lineup and no move within the
lineup; the winner names were one off from the original's two tables; the
manager's fall stopped at storing its screen; the in-game EXIT in the arena
left without its save; a leaving merchant kept his stock and a leaving
specialist his figure.

## Evidence

REA (Exanima.exe 0.9.5), read in the assembly:

- The season reward maker instances the record through the object
  database's dispatcher; when the instance has an item body its quality
  byte is 255, damage and dirt 0, the three packed spreads each set from
  0.0, then the description pass runs. The week's verdict stows the prize
  and each reward with flags 0; the prize is the tournament's copy graded
  pristine when the week is drawn.
- The arena load runs, after the arena chunk, a pass over the arsenal's 25
  boxes: each entry's id through the instancer (an untagged base record
  with the instance bit is cloned into the local tier); a changed entry
  remaps its holders by its 0x400 and 0x200 flags.
- The board's weapon restriction pane is a child built from three records
  of its own (caption, the weapon row's name, the Choose button); the
  compact form moves them up. Choose reads the tournament table's selected
  event, opens the loadout form in mode 3 at the first entrant's row with
  the event's weapon words in the view's hands, and installs a close hook
  that writes the view's first hand pair back to the event (the second
  word's item id cleared) and refreshes the weapon mark.
- The lineup's drop reads the dragged row from whichever table it came
  from, so a lineup row dropped on the lineup is swapped with the row under
  it, or moved to the end when dropped past the rows.
- The winner cell names the field's company c by the name table's entry
  c - 1; the week slot is the field's word plus two and the season table
  names slot s by entry s - 2.
- The match tick on the manager's fall: the heartbeat's fade, the screens
  and the power tree's bar closed, the tip 3 (fatal) or 2, the controls
  blocked, and with a teammate standing the spectate start with the room
  walk forced to the arena's spawn.
- The main menu's in-game EXIT in the arena: the screens closed, the first
  unfit specialist's departure, the save without the death rule, the
  arena's leave, then the transition to the title.
- A merchant's leaving empties his stock; a merchant's or a physician's
  leaving finds the first character with that profession nibble, clears
  its items and removes its scene node.

## What Changes

- `exa_game::arena_rewards`: the reward maker and the won items' kinds;
  `arena_hub_live::hub_stow_won` makes them instances and stows them.
- `save_live::remap_loaded_items`: the load's id pass over the live stores,
  run by the arena slot load before the continue fixups.
- `exa_ui::match_board`: the weapon pane's records, its layout, the Choose
  button's paint, hit and action, and `Board::reorder_team`.
- `hub_forms`: the lineup's open on the event page and its close.
- `career`/`match_board`: the week slot and the winner cell's name entries.
- `arena_match`: the manager's fall's steps and spectate start.
- `flow`: `Flow::arena_exit` and `Step::ArenaExit`.
- `hub_forms::apply_staff_changes` and `arena_hub_live::remove_left_figures`.

## Impact

Lockstep: arena traces change where a season is swept or a week's prize
won (fresh instances in the local tier), on an arena load with base items
in the arsenal (instances and remapped holders), and on a manager's fall
(the spectate session). The saved week slots of a rival week rise by one.
