# Proposal

## Why

The power tree screen, its learning rules, the power bar, the constantly
displaced body and each Force creator's look were first written from a
summary of the original, with invented values in several places: text
indices read as unknown words, a text table one string short, effect
scroll offsets divided by the layer count, invented sound pitches, one
icon sheet for every domain, a frame program that mixed two rows. This
change checks every rule against the disassembly and corrects them.

## What Changes

- The power tree reader keeps the name and description text indices of each
  node. The text chunk's count is its last string's index (one more string
  than the count), and an offset that lies outside the text leaves that
  string empty instead of failing the file.
- The tree screen (`exa_game::thaum::tree`): zoom level, opening view, pan
  spring, hover and focus choice, link angle, press branches, slot select
  and bar drop, the power bar's load, store, cast and toggle, the grant
  routine and learning gates, the default pairs and the domain opening, the
  key handler and the running-power limit.
- The tree and bar draw (`exa_ui::tree_draw`, `exa_game::power_screen`):
  the draw order, the shader parameters of the icon, frame, ring and link
  programs, one icon sheet per domain, and the frame program's band and
  texture rows with its coarse mip level.
- The constantly displaced body (`exa_game::thaum::veil`): the shell
  timeline, the fade lists, the hidden weapon, the hum start in double
  precision and the tendril image.
- Each Force creator's look (`exa_game::thaum::force`, `visuals`): materials,
  parameter block sizes, GI program, sound ids with the pitched plays the
  creators and the starts really make, the noise picture, and the effect
  shader's texture scroll with the global divisor.
- Wiring: the P and C keys open the screen and the bar, the pointer goes
  through the GUI task, the canvas is drawn as a skin batch while the
  screen is open, and the use key casts the slot's power with marker
  entities for the effect nodes.

## Capabilities

### New Capabilities

### Modified Capabilities

- `thaumaturgy`: the tree and text reader, the power bar, the learning
  gates, the constant-displacement veil, the Force creators' looks and the
  effect scroll.

## Impact

- `exa_formats::power_tree` (`PowerNode::name_text`, `description_text`
  replace the two unnamed words), `docs/formats/OTHER.md`.
- `exa_game::thaum::{tree, veil, force, visuals}`, `exa_game::power_screen`,
  `exa_ui::tree_draw` (`Texture::Domain`), `exa_assets` (the tree and icon
  loaders), `docs/THAUMATURGY.md`, `crates/exa_game/README.md`.
- Evidence: REA (Ghidra 12.1.4) on the tree class's reader, load, press,
  hover, key, draw and bar routines, the grant routine, the veil and the
  Force creators, read against the assembly where the decompiler reordered
  float sums, hid sign-bit compares or turned min/max into other compares;
  the effect shader's two scroll callbacks; the draw's fragment programs
  (their sources are kept in `exa_ui::tree_draw`). No corpus file is
  needed; the one real-tree test reads the install and skips when
  `EXANIMA_DIR` is unset.
- Out of scope: the bar's form flags and opening stamps, the running-power
  mark callback, the bar toggle's possessed-body choice and mind-unit
  release, the pre-dispatch GUI call, the link ribbon texture, the
  tooltip, prompt and preview layout, the show/hide fades and the bar's
  open and close animation, the NEW and PICKED ring texture, the zoom pivot
  about the cursor, the base form's press and key handlers, the cast-fail
  sound and queue, the effect geometry and GL submission of the Force
  creators, and a driver that calls the grant routine or the veil.

## Phase

Phase 7, thaumaturgy in play: the power bar and its dispatcher, with the
effect looks of Phase 5.
