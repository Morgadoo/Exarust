# Proposal

## Why

The original's modal screens, popups, pickers, item windows and script
role classes had no counterpart in Exarust beyond the item panel and the
in-game GUI. A cheaper model drafted these from decompiler output; this
change replaces the guesses with rules checked against the instructions.

## What Changes

- `exa_ui`: tooltips, message box, shader warm-up and key-capture screens,
  the game-state transition, the story background picker, the dialogue
  reply list, the creator's picker fields, the item card, container popup
  and parchment, the colour window and confirmation dialogs, the help
  screen, the GUI task stack, the one-time tip banner, the colour swatch
  opener, and the viewport helpers.
- `exa_items`: the off-screen icon renderer, the body loot window, and the
  corrected inventory window and arsenal rules.
- `exa_script`: the `Role` script class on the stored role-list blob, with
  the original's tail placement and overrun on a resized entry.
- `exa_game`: the character creator's pages (base, hair, physique wait,
  hair icon refresh, edit page and its close), the story and arena save
  families, the emblem editor's steps, and the manage form's close.
- Docs: `docs/UI.md` (Retail screens and popups), `docs/RENDERING.md`
  (Item icons) and the crate READMEs.

## Capabilities

### New Capabilities

- `retail-screens`: the forms and rules above, as plain records.

### Modified Capabilities

## Impact

- New modules listed above; `crates/exa_ui/src/game_gui.rs` (journal close),
  `crates/exa_ui/src/menu.rs` and `crates/exa_ui/src/colour_swatch.rs`
  (small additions).
- Nothing new is drawn by the running game; the modules are tested rules
  waiting for their forms. Not wired: every module named here.
- Evidence: the executable's routines for each form, read as assembly
  (`context/re/packs/screens/`), and the retail bytes for the data tables.
  Corrections to the draft are listed in the design.
- Out of scope: painting any of these forms, the creator's 3D preview, the
  save-package section delete (save-package code), the role scripts' use of
  `ScriptRole` (scripts keep `builtins::Host`), and the roles that the
  assembly leaves unnamed (they keep neutral names).

## Phase

Phase 7, game modes and content: the UI section (forms and screens); the
item windows also belong to Phase 4, combat and items.
