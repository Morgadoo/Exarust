# Proposal

## Why

Every run logged `GUI texture "Rayform10R" / "Rayform08R" not found`, and
every GUI text drew with the base font's atlas stretched to the level
font's quads: the original's five interface faces (`Rayform06R` …
`Rayform16R`) are not in any pack. The original keeps them as raw-data
resources inside `Exanima.exe` and its font records name them as module
resources (`Rayform10R_rfi`), as do the glyph and symbol sheets
(`GlphCommon_rfi`, `GUISymbl_rfi`).

Round 56 moved the inventory to the game skin's own form draw. The
journal, the loot window, the container popup, the item card and the
hub's windows still painted with the default skin's background and their
raw constructor words, so they had no title bar, shadow or ornamented
frame, and their panels stacked a second fill.

Phase: Phase 6 (story mode: the in-game screens) of docs/PLAN.md; no
PLAN.md edit in this change (the owner's).

Evidence (REA, Ghidra on `Exanima.exe` 0.9.5, checked against the
assembly): the image reference resolver's module-resource branch (a
system resource lookup by name, raw-data type, case-insensitive), the
font records' references, the executable's resource tree; the base form,
journal, loot window ("Body"), item panel, item card, container popup and
trade window constructors (their style and attribute words); the hub's
record forms' words were already traced. Capture: the retail inventory at
1592 × 1046 from round 56, compared with Exarust at the same size.

## What Changes

- `exa_rpk::ModuleResources`: a total parser of a PE image's named
  raw-data resources. `GameData` reads them from the installation's
  `Exanima.exe` at open time; `GameData::texture` falls back to the
  resource `<name>_rfi` after the packs, so the GUI loads the Rayform
  faces (and any other module image) by their bare names. The GUI loads
  all six faces at start-up.
- `exa_game::gui_paint`: `FormWords` with the traced constructor words of
  the journal, loot window, item panel, item card, container popup,
  character view and trade window; `paint_top_form` and
  `paint_held_form` over the line skin's form draw with committed words.
- The journal, loot window (and its two item panels), container popup (and
  its item panel), item card, the hub's record forms (Manage Company,
  Company Records/Status, dismiss and yes/no dialogs, loadout, the board)
  and the trade window (its character view and four item panels) paint
  through it: shadow and backdrop under the children, title and frame over
  them.
- `--open-loot FRAME`: a debug helper opening the nearest dead body's loot
  window, for unattended checks.

## Out of scope

- The tables of Manage Company and the board's panes and pictures (their
  classes' words are not traced; they keep the default background).
- The settings, save, help, creator and arsenal forms (other skins or
  not asked for here).
- The item card's title tint and the card's own text layout.
- No game data is committed: the faces are read from the installation.
