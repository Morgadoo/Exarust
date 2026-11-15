## Why

Roadmap task B1 (wiring lane `wireskills`): the skill screens, the icon
manager and the table view. Findings F76 and F77 of the UI audit: no key
reached the live skill screen, so only the right button closed it, and the
open routine's toggle was skipped. The table view (`TGUITableView`) had a
full port with test callers only, while the arena's tables (the match board
and Manage Company) ran hand-written copies of its press, drag and drop,
drew no selected row and no dragged row's label, and let a greyed tier be
selected.

## Evidence

REA (Exanima.exe), checked against the assembly where constants or order
matter:

- The skill screen's key method (slot 37): with the compact flag clear, on
  the ring page, unlocked, outside the reveal's first 100 ms (the reveal
  deadline less 400 against the GUI clock) and not hiding, any key calls the
  open routine's toggle; the open routine with a screen up only closes it.
- The table view's press, hover, drop and drop-callback slots, its draw's
  selection branch (the faded rectangle at a quarter strength from the
  highlight group's fifth and first shades, then the selection band, hovered
  or not) and the drag label callback (the form group's fifth and first
  shades, the unselected band, the label group's ninth shade).
- The two constructors that build table views (the match board and Manage
  Company): column widths, owner-drawn columns, the drag kind 0xab01ac00 on
  the company, lineup and roster tables, the pad activation words, the
  drop and accept callbacks, the roster's first row selected; no column is
  editable or a choice column and no table grows (no instruction sets the
  growing flag on a form), so the edit field, the choice list and the added
  row never run in 0.9.5.

## What changes

- `exa_game::skill_screen`: the pointer system hears the keyboard while the
  screen is up and closes it through `exa_ui::skill::leave_skill_ui`; a skills
  request while it is up closes it.
- `exa_ui::table_view`: `hover_at`, `selection_band`, `paint_selection`,
  `paint_drag_label`; `owner_query` runs a mutable drop callback.
- `exa_ui::match_board`: `TableLayout::view`, `Board::table_view`,
  `Board::press_table` (the board's table presses go through the view).
- `exa_game::hub_windows` / `hub_forms`: the board's company and lineup
  tables and Manage Company's tables press, drag and drop through their views;
  every table draws its selected row's band and a moved drag its label.
- `--board-table hold:ROW` keeps a company row picked up.

Not in: the pad's menu dispatch (the table's command slot and the skill
screen's pad keys and link search), the arena loadout form's skill page (the
only page-2 skill window and the hide fade's caller), the creator's compact
skill tree (DEC-3), the icon atlas's turned and mirrored draws (their
callers are the skin's centred icon and the line skin's icon, uncalled).
