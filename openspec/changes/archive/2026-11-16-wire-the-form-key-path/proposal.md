## Why

Roadmap tasks B1 and B4 (lane `guiform3`): the base form (TGUIForm) had
its key slot only on the hub's Tab. The key a GUI window hears was not
modelled end to end: the window procedure's mask word, the focused
record's key message for every record class, the forward to the parent's
key slot, a key slot on the game screens and the manual, and how a key
the GUI does not take reaches the player's hotkeys.

## Evidence

REA (Exanima.exe), checked against the assembly:

- The key-down hands the key and the modifier word to the focused form's
  key slot. The first key-down of a key with Shift, Ctrl or Alt held
  recomputes the word as Shift 0x20, Ctrl 0x40, Alt 0x80; with none held it
  keeps the word; a repeat sends the kept word with 1. A key-up recomputes
  it one bit lower (Shift 0x10). A typed character above 0x1f goes to the
  focused form with the word 2.
- The base form's key slot: with a focused record, Tab steps among the tab
  stops (back only on the word's 0x10); any other key is the record's key
  message 0xa0 with the key and the word, taken when the answer has both
  low bits (then a redraw); a key nobody took goes to the parent's key slot
  unless the word has a low bit.
- Only the text field and the combo box have an arm for 0xa0. The button,
  image button, check box, label, trackbar, icon button, picture,
  separator and swatch handlers return their zero result for it.
- The 3-D view's key slot runs its key handler (the player's hotkeys) and
  then the parent's key slot. The game windows are added to the current
  task, not to the view, so a key on a window never reaches the hotkeys.
- The activation calls the focus routine only for a nonzero control; the
  openers and the show pass 0. A new inventory's and a new journal's open
  end with the forced pointer update, as every hide does.

## What Changes

- A key mask model (`exa_ui::gui_keys`) and the frame's key list
  (`exa_game::gui_keys`).
- A key slot on the window manager's forms, the base form's from its
  records, and the window manager's key route up the parents.
- The game screens and the manual hand the frame's keys to the focused
  form; the player's hotkeys hear a key-down only while no game screen has
  the focus.
- The hub's Tab takes the key-down's word: Shift with Tab steps forward.
- The openers' activation leaves the key focus with the form under the
  pointer; new windows and hides end with the forced pointer update.

## Impact

`exa_ui::{gui_keys, gui_core, base_form, gui_controls}`,
`exa_game::{gui_keys, gui_screens, help_live, option_forms, hotkeys,
arena_hub_live}`. With the pointer over a game screen the player's hotkeys
(I, J, Tab, R, ...) do nothing, as in the original; Shift+Tab on the hub's
forms steps forward. No simulation change.
