## Why

Roadmap tasks B1 and B4 (lane `guikeys`): after the base form's key path
(guiform3) the main menu, the power tree, the settings form, the hub's
text fields, the skill screen and the destination screen still read
Bevy's keys themselves, and the `KeyDef` bindings fired from every key
whose modifiers were held (F135). The skill screen and the destination
screen heard only some keys (F76, F122).

## Evidence

REA (Exanima.exe), checked against the assembly:

- The key-down handler sends each key with the modifier word to the focus
  form's key slot; the key-up does not reach any form.
- The task's key slot ignores a word with either low bit, passes Shift
  (0x10) on without a lookup, else scores each entry for the key (every
  required modifier held in the global word, one point each, plus one)
  and runs the strictly best (the earlier on a tie) with the layer as the
  current task; a key with no entry goes to the parent unless the layer is
  the root.
- The shortcut table's two writers: the `KeyDef` setter (the first six
  macros, always with a modifier, on the root task) and the game boot
  (Escape 0x1b with no modifier on the root, the main menu's toggle).
- The main menu's key slot (shared with the end-session menu): a word with
  bit 2 shifts the ten-byte name buffer, appends the byte (A-Z lowered)
  and on `barechoons` toggles the menu music; any other word goes to the
  base form's key slot.
- The power tree's key slot compares the key with the bound next-power
  and previous-power slots' 16-bit words, not fixed arrows.
- The skill screen's and the destination screen's key slots read neither
  the key nor the word.
- The settings form keeps the base form's key slot.

## What Changes

- The root task's key slot (`commands::root_task_keys`) with the bound
  `KeyDef` lines and Escape, fed by `gui_keys::TaskKeys`: the game view
  after the hotkeys (`commands::view_keys`), the game screens, the main
  menu and the settings form. Escape on the open menu goes back or closes
  it (`menu_screen::menu_toggle_key`).
- The main menu, the settings form, the power tree, the skill screen, the
  destination screen and the hub's text fields read the window
  procedure's keys (`GuiKeyFrame`).
- Scripted presses run before the frame's key reader (`GuiKeySet`).

## Impact

`exa_game::{commands, gui_keys, gui_screens, menu_screen, settings_form,
power_screen, thaum::tree, skill_screen, destination, option_forms}`,
`exarust::viewer`. `KeyDef` bindings fire only when their key reaches the
root, one per key; the tree cycles on the bound power keys; held keys'
repeats and typed characters now close the tree, the skill ring and the
destination screen. No simulation change.
