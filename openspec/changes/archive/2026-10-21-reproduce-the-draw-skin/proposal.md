# Proposal

## Why

Every form and control of the original is drawn by its base draw skin
(`TGUIDrawSkin`): the colour scheme and its derived shades, an
immediate-mode batch of quads, lines and triangles, and the parts built
from it (faces, panels, marks, sliders, scroll bars, form chrome, the
default form background, the page's draw order and the labels).
Exarust had the skin's opening metrics, hit tests and a few parts; the
rest of the scheme and the drawing was missing. This change advances
Phase 7, milestone "Full UI using the original GUI skin, icons and
manual pages" (docs/PLAN.md).

## What Changes

- The scheme: four colours expanded into twelve shades each, the accent
  and its mixes, the hot copy of the form group and the two alpha ramps.
- The layout metrics the constructor and the unit set.
- An immediate-mode batch: program selection (the selected mode binds
  once), blend word, ink, texture offset, and runs submitted as quads,
  lines or triangles into one shared vertex list.
- The primitives: outlines, four-stop and alpha-scaled gradients, striped
  bars, selection bands, rounded fills and frames, line frames, bevel and
  gradient boxes, shadow and bevel rings, clamped gradients and the two
  scroll arrows.
- The control parts, the form's marks and their icons, the form chrome,
  the default background, the page's clip, scroll and draw order, and the
  label placements, inks, glow and fade.
- The skin's programs in the original's order (flat, rounded, textured);
  the duplicate frame switch is replaced by the style frames.

## Evidence

REA (Ghidra) listings of the skin's 73 functions in Exanima.exe 0.9.5,
with the out-of-group helpers they call (colour scale, tone and mix, the
minimum, the vertex, begin and end helpers) read at the instruction
level for float order, rounding and NaN behaviour. No corpus field is
involved: the scheme table lives in the executable.

## Out of scope

- Submitting the batch to a render pass: the menus and the HUD keep their
  present painters, so nothing on screen changes.
- The textured subclasses (the `guiskin.rfi` atlas and the HUD's stone
  skin) and their shaders.
- Rasterising text and icons: the model returns their places, inks and
  scales.
- The string release the form draw ends with, and the GUI globals that
  rescale with the unit (the skin reads them at their static values).
