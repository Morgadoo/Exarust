# Proposal

## Why

Phase 7 (Game modes and content) carries the thaumaturgy tree screen
(`TThaumTree`, 48%). A DeepSeek lane (thaumtree4) added the draw head's
zoom blend about the cursor and its eased opening scale, the links as the
spline renderer's ribbons, the skin font's glyph quads, the screen's UI
sounds, the form's creation fields, a domain wheel child, the technique
scan over funded slots and the sheet bindings. The review read each rule
in the instruction listing and found the first zoom level's unordered
compare inverted, the slot click reading the focus instead of the stored
node, a third copy of the grant routine, a guessed wheel sector hit that
duplicated the existing `effects::wheel` and switched to any domain on a
click anywhere in the circle, the record's wheel shares overwritten when
the child was created, the opening scale taken about this frame's point
instead of last frame's, the glow accent's alpha scaled with its colour,
and the spline width term summed in double.

## What Changes

- The draw head blends the zoom spring's last two values by the frame's
  share of a 17 ms step, rescales the pan's goal, position and previous
  point about the cursor by the blended zoom's ratio to the last shown
  one, and scales every drawn node about the first node's previous placed
  point by `ease(1 − r²)` while an opening or domain fade-in runs.
- The links are spline ribbons with start and end handles, the join term,
  the dim alpha and the three `lnwdt` terms (single-precision first term).
- The domain wheel child is `effects::wheel::Wheel` placed at
  `(10, height − 290)`: its icon hit test and confirm (known, different
  domain) start the domain switch; its eased value is the focus share and
  its show the value the draw head writes into it.
- The first zoom level answers −8 below its threshold; a slot click takes
  the stored node; the dragged link's reach is squared in double; the
  duplicate saved-power check is removed (`grant_power` is that routine).

## Evidence

The draw head, the placement and stretched-link routines, the zoom level
routine, the press, drag and wheel handlers, the wheel share refresh, the
cost/known-domain routine, the grant routine, the bar cycle, the tree's
skill check and the spline renderer's width setter, read in the
instruction listing; the wheel's constructor and open routine and the
spline width constants (4, 0.5, 0.3, 0.15, 6) read from the executable.

## Out of scope

The wheel's own ring, icon and label draw (its sheet is shown), the GUI
form class, the skin's panel chrome and label colour, the active-power
mark callback on bar load, GPU submission of the tree's draw (it is still
rasterised on the CPU), and the "not now" slot retry.
