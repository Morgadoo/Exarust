## Why

The goal is the whole original, including code the retail game never runs.
Three parts of the GUI toolkit were at 0%: the popup menu, the tree view and
the object appearance editor's handlers. Only the developer editor uses them
(the taskbar menus, the open and save dialog, the appearance panel), and the
retail game never opens that editor.

Phase: 7 (interface) in docs/PLAN.md, the remaining original widgets.

## What Changes

- `exa_ui::popup_menu`: the popup menu's layout, item hit, shortcut labels,
  submenu chain (hover opens at once, flips at the screen edges, hands the
  pointer between levels), press and release rules, and its three-pass draw.
- `exa_ui::tree_view`: the tree view's pre-order rows, hit codes (rows and
  chevrons), hover selection, press, keys, rename field, scroll, drop check,
  drag label and four-pass draw, including popup and compact modes.
- `exa_ui::object_editor`: the appearance editor's control handlers (levels,
  variation steps, layer controls, colours, presets, seeds) and the palette
  popup's placement.
- `skin::PaletteInk`: names a palette shade by group and index.
- None of these has a live caller, as in the retail game.

## Evidence

REA (Ghidra) on Exanima.exe: the classes' constructors, virtual slots and
walkers, read as pseudo-code and checked against the assembly where Ghidra
dropped or reordered values (the separator's row offset, the hit test's
chevron zone and result, the walkers' helpers).

## Out of scope

- A caller: no editor form is built, nothing opens these controls.
- The appearance editor's form construction, preview rebuild and item
  container refresh, and the material tables its lists are filled from.
- The tree view's unit routines (the file dialog around it).
