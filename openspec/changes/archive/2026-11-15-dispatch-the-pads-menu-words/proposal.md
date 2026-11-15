## Why

Roadmap tasks B1/B4 (wiring lane `menupad`): the gamepad's menu dispatch.
The pad reader threw away the menu reader's key word and no form sat on the
focus stack, so no form's pad slots ran: the forms' key slots (the main
menu's, the manual's, the skill screen's, the arsenal's, the table views'),
the skill screen's link search and the table view's centre-and-default, and
the set switch between the menu set and the gameplay layer.

## Evidence

REA (Exanima.exe), checked against the assembly where constants, masks or
order matter:

- The menu key dispatch: the focused form's key slot with the cursor; an
  unanswered word (0x40) goes whole to the focused control when the form's
  control-keys bit is set, else `MenuBak` to the form's back slot when its
  attribute word has bit 2, a navigation step (bit 1) to the directional
  pick, and the rest masked with 0xf322 to the focused control.
- Sending a key to the focused control: a form without a control list gets
  its own key slot, else the record's handler with message 0xc0, after which
  the record's 0x200000 bit becomes the form's control-keys bit.
- The directional pick: forms without a control list are placed through
  their slot 31 (the table view's centre-and-default) and scored as
  controls are.
- The skill screen's link search (slot 31): the in-place normalisation of
  the offset before its `y` is read for a sideways step (0.1 beside the hit
  level in double precision, 0.25 in the scan).
- The message box's key slot: 0x100 closes, 0x600 closes with -1, any other
  word is answered.

## What changes

- `pad_menu`: the focus stack follows the live forms' shows and hides
  (main menu, settings, manual, skill screen, arsenal, hub windows); each
  push activates the menu set, the last pop restores the gameplay layer.
  The menu reader's word goes to the top form; the gameplay sets read only
  their own actions; `MenuExit` runs the main menu's toggle (exit hook).
- The forms' pad arms: the main menu's key slot, the manual's two-pane key
  slot, the skill screen's key slot and link search, the arsenal's scroll
  words, the Settings form through the base dispatch (pick, focused
  record's key message, back slot), the arena board's tables (pick by
  centre-and-default, command slot, activation words) and the hub's message
  box.
- `--pad-press` gains the menu set's buttons and both menu sticks.
