## Why

Roadmap task B4, lane `ui2`: the remaining rows of the triage's port lanes
L14 (UI game screens and settings) and L15 (UI toolkit and input), after
round 303 cut its two UI lanes short. The combo box's list was placed by
the open/save screen's row cut instead of the GUI's child placement; a
hovered combo box drew its text as an idle one; the pad's pick reached
records by an approximation; the pointer table held platform arrows where
the shipped executable loads none; the first built-in glyph was always the
fallback cross; and the game skin's atlas was drawn at its file size
instead of the 40 by 20 units the original resamples it to at each GUI
unit.

## Evidence

REA (Exanima.exe 0.9.5), the assembly read where order and constants
matter: the combo box's list open and the child placement it calls; the
drop-down list's layout slot; the combo box's front draw and its text
alignment; the pad's reach test; the pointer table's loader and the
executable's resource directory (no cursor resources by those names); the
glyph builder and its `GUISymbl_rfi` branch (the 48-pixel square at
(0, 48) copied into the glyph format, then added to the small atlas); the
game skin's load (`guiskin.rfi` resampled to unit x 40 by unit x 20).

## What Changes

- The combo list opens a pixel over its owner's bottom edge, or above it
  with its bottom a pixel under the owner's top; it is never cut.
- A hovered combo box draws its whole text through the skin's glowing text.
- The pad reaches a record by the original's test (shown, enabled, pad
  style bit, focusable class).
- Pointer slots 0-4, 9 and 10 are empty; an empty slot hides the pointer
  image.
- The first glyph is the executable's GUISymbl square.
- The game skin's atlas is resampled to 40 by 20 units at the GUI unit and
  again on a scale change.

## Impact

Pictures: every form drawn from the game skin's atlas samples the resampled
atlas (golden scenes with GUI forms change). No simulation change.
