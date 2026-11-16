## Why

Roadmap task B4, lane `uiscreens` (the triage's L14: UI game screens and
settings). Several screens and settings rules differed from the original:
the text box's line count, the forget dialog's lines, the controller value
Apply writes, the binding file's rows and controller word, the Controls
page's key names, the icon render's light preset for weapons, the GUI's
reselect on game mode switches and the Controls page's view tabs.

## Evidence

REA (Exanima.exe 0.9.5), read in the assembly:

- The text box's line count: a text under two bytes is one line; one more
  line when the byte before the last is a carriage return.
- The skill screen's forget dialog draws its title and question with the
  centred line draw in its 82-unit width; Yes and No with the plain line.
- Apply writes `InputMode` as the one-character string `3` for the
  controller, `0` otherwise.
- The binding saver writes the controller word the loader read and the
  page's saved rows as they are; the controller switch never writes the
  word. The key cells are named from the page's rows.
- The icon render keeps the cache variant's top two bits, adds the effect
  bit for model flags `0x2000` and the weapon bit for a weapon section.
- The Character and Spectate modes' start and end routines end with the
  GUI's forced reselect.
- The settings page switch shows the view tabs when Steam Input has a
  controller handle.

## What changes

- `exa_ui::gui_text::count_lines` follows the text box's count.
- The skill draw's labels can be centred in a box (`centred_in`).
- `InputMode` is written as `3`.
- `Bindings` keeps its saved rows and controller word; captures write the
  row as it is; `row_key_name` names the cells.
- `item_icon::render_variant` builds the render variant; `ItemModel`
  carries `weapon_section`.
- `player::reselect_on_mode_switch` reselects the game GUI task.
- The settings form sets the view tabs from a connected pad.
