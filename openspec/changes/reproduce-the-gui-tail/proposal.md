# Proposal

## Why

The original's remaining GUI classes had no counterpart: the top-level
layer, the colour picker, the combo box's popup list, the palette swatch,
the character creator's tone and hair pickers, the skill atlases, the
manual and the one-time tips, the trade window, and the tail of the item
view, the icon manager and the chase camera's wall probe. A first pass
written largely from the decompilation carried misread tables (the tone
field's bytes, the hair hue run), guessed colour conversions, invented
words, layouts and strings, rules duplicated from existing code, and a
wall probe that drifted the live camera's pitch. This change keeps the
verified rules and corrects the rest against the instructions.

## What Changes

- `exa_ui::gui_task`: the layer's constructor, shortcut scores and
  forwarding, draw alphas, hit test, callback list and focus records.
- `exa_ui::colour_picker`: one picker per swatch, the guarded RGB and
  HSV refreshes, the write-back and the revert source;
  `exa_formats::palette::rgb_to_hsv` (the engine's helper).
- `exa_ui::drop_down`: the popup list's words, layout, hit, press,
  release, command, close and selection band.
- `exa_ui::col_swatch`: the palette popup on the lifted palette ramp.
- `exa_ui::pickers` and `exa_assets::human_skin::{tone_linear,
  tone_blend_target, tone_field_bytes}`: the tone and hair pickers and
  the picker base's pick.
- `exa_ui::skill_gui` and `exa_ui::skill::SkillScreen::{open, attach}`:
  the skill atlases, the page ring's stretch and the record attach.
- `exa_ui::help_system`: the manual and the tips.
- `exa_ui::item_view`: the select, drop, rename-caret, marker and
  selection-fill slots.
- `exa_items::icon_cache`: the manager's setup, release, refresh, pixel
  tests, apparel size and user-item errors.
- `exa_game::shop_window`: the trade window's layout and balance row.
- `exa_game::nav_camera` and `player::ChaseCamera`: the per-tick wall
  probe (unanswered live), the pitch ease, eye blend and field of view.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: layer, colour picker, popup list, palette swatch,
  creation pickers, skill atlases, manual and tips, item view slots.
- `shop-trade`: the trade window.
- `player-movement`: the camera's wall probe.

## Impact

- Code as listed; `docs/UI.md`, `docs/PLAYER.md`, the `exa_ui`,
  `exa_items` and `exa_game` READMEs.
- Evidence: reverse engineering of `Exanima.exe` 0.9.5 with REA
  (Ghidra): the listed classes' routines instruction by instruction,
  their data tables and strings, the engine's colour, byte-pack and hex
  helpers, the tone table's start-up routine and the picker base's pick.
  No corpus files.
- Out of scope: drawing and wiring the new forms into the running game,
  the hair material's shading routine (ported with the character
  generator), the picker base's marker draw and gamepad nudge, the colour
  picker form's own layout and number-field format, the manual's
  memory-block loading, the item renderer and apparel painter behind the
  icon manager, the wall probe's scene sweep, the trade window's panels
  and Trade button.

## Phase

Phase 7, game modes and content: the interface's remaining original
widgets.
