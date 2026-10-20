# Proposal

## Why

When the GUI unit changes (the window's height or the GUI size setting),
the original's main GUI unit rescales every form and every control on
it from the old unit to the new, keeping their anchors; it also fills
the large icon atlas from the common glyph sheet and builds its message
boxes from a fixed layout. Exarust's forms are laid out once at one unit.

## What Changes

- A form rescale: border, place, size, content height, minimum, child
  forms and control records, by the original's anchor rules and its
  16-bit record arithmetic (odd widths under 16 units and odd heights
  made even, the vertical centring correction).
- The glyph sheet's nineteen 64-pixel icons in the large atlas's order.
- The message box's layout: width, label, buttons, height and top.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: form rescale, glyph sheet, message box layout.

## Impact

- `exa_ui::gui_rescale`.
- `docs/UI.md` (Forms) and the `exa_ui` README.
- Evidence: reverse engineering of `Exanima.exe` 0.9.5 with REA (Ghidra):
  the main GUI unit's form rescale, its glyph-sheet loader and its
  message box builder, and the form's client layout. Addresses are in
  `context/re/gui-widgets.md`. No corpus files.
- Out of scope: each control's own resync message after the rescale,
  the form hook that runs after it, reading `GlphCommon` from the pack
  (the atlas takes icons at its cell size), the message box's form and
  its buttons' handlers, the pointer and tooltip dispatch of the unit.

## Phase

Phase 7, the interface: forms that follow the GUI scale.
