# Proposal

## Why

`exa_ui::skin` builds the draw skin's primitives into a batch, but nothing
submitted a batch to the screen, so no form was ever seen: the inventory and
journal opened as forms on a `GuiTask` and stayed invisible. A first wiring
also guessed at words the original does not write (a title bit, a grey
wash over the character view, preview bars, a tab field) and mis-copied the
pack's attribute word.

## What Changes

- `exa_ui::gui_draw`: a frame is an ordered list of layers (a batch, the
  texture it samples, a clip). A CPU rasteriser reproduces the skin's
  programs (solid, textured with `TexCol`, rounded with `Scale`, and the
  mode a subclass answers: flat for the line skin, the stone for the HUD's),
  the ten-entry blend table (shared with `exa_ui::blend`), the blending-off
  word, a blend word of 0 as the steady 0x203, and a top-left fill rule so a
  blended quad diagonal is drawn once. The canvas image is kept when a frame
  equals the last.
- `exa_ui::line_skin`: the shipped line scheme and its palette.
- `exa_game::gui_paint` and `gui_screens`: the inventory and journal paint
  with the attribute, frame-style and spacing words their constructors
  write (no title bar), the pack panel with its container's words
  (attributes 0x130032, style 0x80), and the control records laid out by the
  two-pass layout: Encumbrance label, Primary, New and Erase, with captions
  placed by the skin's own caption rule. The base font's atlas stands in while
  the Rayform faces are not shipped, drawn at the level font's metrics.
- Removed from the first wiring: the invented title bit, grey wash, preview
  bars and tab field.

## Capabilities

### New Capabilities

- `gui-draw-path`: submitting the draw skin's batches and painting the
  in-game screens.

### Modified Capabilities

## Impact

- `exa_ui::gui_draw`, `exa_ui::line_skin`, `exa_ui::skin` (a `subclass`
  field), `exa_game::gui_paint`, `exa_game::gui_screens`, `exarust`
  (`--debug-gui-form`), `docs/UI.md`, the `exa_ui` and `exa_game` READMEs.
- Evidence: REA (Ghidra 12.1.4) on the skin's blend setter and mode
  dispatch, the form constructors' attribute and style words, the inventory
  and journal constructors' control records, the line skin's scheme record
  (read from the executable's data: four colours with scale and tone, the
  accent, and alpha bytes 170 and 125), and the caption placement. Tests are
  synthetic; a headless run on the RTX 3090 draws the screens.
- Out of scope: the character view and its preview bars, the pack's item
  icons and bulk, the unlocated second inventory record, the controls'
  callbacks, the tab strip, the journal's entry list and typing, the loot
  window, the technique selector, and the Rayform faces.

## Phase

Phase 7, game modes and content: the in-game GUI unit and the draw skin.
