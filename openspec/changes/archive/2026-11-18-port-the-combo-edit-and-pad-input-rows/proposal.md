## Why

Roadmap task B4, lane `input3`: the port queue's lanes Q11 (UI toolkit)
and Q12 (input, keys and pad focus). The journal's entry list is an
editable combo box, but its press always opened the list and no key
reached it, so an entry could not be renamed (F89); entering combat mode
closed the screens with an item drag still in flight; an open combo list
or palette heard no pad words; and the pad's pick on the arena board
reached its tables only.

## Evidence

REA (Exanima.exe 0.9.5), the assembly read: the combo box's message
handler (pointer query, press, drag, release, key and focus arms), its
text draw-through (the caret draw with the focus bit and the form's caret
bit), the journal list's callback (its edit-state arm), the form's hit
(the record answer's hover bits and cursor byte), the GUI press's focus on
a 0x15 answer, the combat entry's release of the GUI root's held left
button before the screen sweep, the drop-down list's and colour palette's
command slots with the list's focus push, and the board's bottom button
records (the pad focus style).

## What Changes

- An editable combo box answers the text cursor left of its arrow; the
  hover's edit state lets a press place the caret, any other press opens
  the list (on the press, not the release).
- The journal's list takes the key focus from New (whole title selected)
  or a press on its text; typing renames the entry and its title row; a
  drag selects; its caret blinks and draws.
- Entering combat mode releases the held left button first: an item drag
  in flight drops at the pointer.
- The pad's words reach an open combo list or palette first.
- The board's reachable bottom buttons are pad pick candidates; MenuSel
  clicks the focused one.

## Impact

Pictures: the journal's list draws its selection and caret while focused.
Lockstep traces: none (GUI only). Code: `exa_ui` (`gui_controls`,
`base_form`, `record_form`), `exa_game` (`gui_screens`, `gui_paint`,
`option_forms`, `hub_windows`, `arena_hub_live`, `panel_live`,
`hotkeys`).
