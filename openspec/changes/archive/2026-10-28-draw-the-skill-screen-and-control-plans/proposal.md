# Proposal

## Why

The skill screen had its placement, hit and popup rules but no draw list,
shader programs, open or close; the extended controls had their message
rules but not their draw calls, the spinner's drag capture or the text
field's and combo box's message dispatch. A DeepSeek pass added them; the
review against the assembly corrected the draw's gate, vertex order,
background, spokes and popup inks, the programs' projection, the spinner's
set word and drag capture, and the skin calls' float alphas.

## What Changes

- The skill screen's draw list: the page-2 background, the opening gate
  and fade call, the dim, the node plates, the textured spokes, the node
  and group icons, the orbit bars, the ring tooltip, the learning row and
  the forget dialog, with the original's single- and double-precision
  sums and the engine's packed sine and cosine.
- The two skill programs (ring and bar sources, the fixed `MPrj`
  matrix, `Colour`, `Tex`, `Prog`) and the reveal sound; the open/toggle
  and close plans.
- The extended controls' draw plans (text field, combo box, spinner,
  progress bar, separator, image, texture view, trackbar, scroll bar,
  caption), the spinner's set message, drag capture and hot halves, and
  the text field's and combo box's message dispatch.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the skill screen's draw and programs, and the extended
  controls' draws and spinner drag.

## Impact

- `exa_ui::skill_draw` (new), `exa_ui::skill_gui`, `exa_ui::skill`,
  `exa_ui::controls`, `docs/UI.md`, the `exa_ui` README.
- Evidence: REA/Ghidra on `Exanima.exe` 0.9.5: the skill screen's draw
  body, group-icon and orbit-bar draws, three tooltip bodies, program
  setup, open and close; the skin's mode switch and the engine's sine and
  cosine helper; the shader sources, the projection matrix, the colour and
  texture-coordinate tables read from the image; the spinner's drag and
  message routines and the progress, trackbar and scroll-bar draws. No
  corpus files.
- Out of scope: mounting the skill screen or the extended controls in a
  running Bevy form (`Hotkey::Skills` stays unhandled), the glyph pass and
  the skin primitives' bodies, technique descriptions, the number
  formatter, the text-edit unit's editing, Steam Input and the audio call.

## Phase

Phase 7, game modes and content: the character's skill screen.
