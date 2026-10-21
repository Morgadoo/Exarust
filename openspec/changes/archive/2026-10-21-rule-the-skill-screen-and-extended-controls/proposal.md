# Proposal

## Why

The skill screen could place its ring and hit its nodes, but not decide
which techniques may be learnt, learn or forget them, spend the screen's
experience or draw its icons, orbits, tooltips and popups; and the extended
controls (text field, combo box, colour swatch, spinner, progress bar,
separator, image and texture views, caption, trackbar, scroll bar) had only
their spans and trackbar map. A first pass at both, reviewed against the
original's assembly, guessed several rules (every node without a
prerequisite marked absent, the level written as the technique, sine and
cosine swapped on the ring, NaN clamped to 0, an invented text-edit loop)
that this change corrects.

## What Changes

- The learning row's requirement mask from the screen's node-present table
  and the record's prerequisite table, both prerequisites funded.
- A ring node's press state (locked, learnt, open), the screen's own
  unlearning rule with its committed points, the learning commit, the
  experience pass and the ready flag; the record's surplus query and
  prerequisite lookup are the record's.
- The ring, satellites and link search put the engine's sine across and the
  cosine down, with the original's angle sum and the stretch on x only; the
  forget arrows are hit before the learning row; the row mask is indexed by
  the slot less one.
- Group icons, orbit bars, node spokes, the icon atlas quad, the hover,
  learning-row and forget-dialog boxes, the popup fade and the screen fades.
- Each extended control class's message rules: creation hooks, the unit
  hook, the edit state and key dispatch, drop-down placement, colour drag,
  spinner commit, press, keys, drag step and buttons, progress, separator,
  image, texture, caption, trackbar and scroll-bar geometry, with the
  unordered clamps that let a NaN through and `CVTSS2SI`'s indefinite
  result.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the skill screen's rules and the extended controls.

## Impact

- `exa_ui::skill`, `exa_ui::controls`, `exa_character::record`
  (`technique_prerequisites` made public; the guessed screen helpers
  removed), `docs/UI.md`, the `exa_ui` and `exa_game` READMEs.
- Evidence: REA/Ghidra on `Exanima.exe` 0.9.5: the skill screen's 29
  methods and the extended controls' 54 routines, read against their
  assembly (float compare order, NaN paths, conversion widths); the
  screen's node table, ring order tables and preview skill, the
  registration records and the engine's sine-cosine helper read from the
  image; the record's surplus query and the experience system's two
  conversions outside the group. No corpus files.
- Out of scope: the skill screen's draw list, shaders and reveal sound, its
  close and player-control restore, the technique descriptions; the text
  edit unit's own editing, the drop-down list form, the spinner's drag
  capture and the colour picker; a Bevy form that mounts either.

## Phase

Phase 7, game modes and content: the character's skill screen.
