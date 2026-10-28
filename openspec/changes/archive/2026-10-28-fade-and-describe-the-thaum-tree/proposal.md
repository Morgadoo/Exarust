# Proposal

## Why

Phase 7 (Game modes and content) carries the thaumaturgy tree screen
(`TThaumTree`, 42%). A DeepSeek lane (thaumtree3) added the screen's
window and domain fades, the bar's opening and closing animation, the
view's 17 ms frames, the stretched link, the tooltip and label layout and
the opening sound, and a "saved power" reader. The review checked each rule
against the original's instructions and found the opening fade inverted,
the bar's eases the wrong function, the bar's closing not run alongside an
opening, the tooltip's texts swapped and its metrics a fourth of their
size, the stretched link drawn from the node's file position, and the
saved-power reader a second copy of the grant routine.

## What Changes

- The tree's opening fade rises from 0 to 1 (`r = 1 − (1 − left·0.001)`,
  window `min(2·(1 − r²), 1)`, bar child `1 − r²`); the 256 ms closing and
  the domain switch's out and in fades.
- The bar's 900 ms opening and closing, run together while both are
  flagged, with the GUI's smooth step `(3 − 2x)·x²`; the tree draw's learn
  and flash fades take the same smooth step.
- The tooltip: the name caption at once (200 ms fade), the wrapped
  description box after 601 ms (300 ms fade), at the original's places;
  label mode draws every node's name instead.
- The stretched link starts at the placed node; the spline's end handles
  are not curve points.
- The duplicate saved-power reader and re-validation are removed; the
  grant routine (`grant_power`) is that routine.

## Evidence

The tree screen's draw head, its tooltip and label routine, the bar draw,
the bar's open, close and toggle routines, the stretched link routine and
the grant routine, read in the instruction listing; the smooth step's
constants (3 and 2) read from the executable.

## Out of scope

The GUI form object, the domain wheel child, the skin's own text draw and
letter sheet, UI sound playback, the zoom blend about the cursor in the
draw head, the opening scale the draw head eases, the spline texture, and
renaming the existing "Tree show" requirement (it describes the hide).
