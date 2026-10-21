# Proposal

## Why

The original's gamepad support is Steam Input: a menu action set and two
gameplay sets whose actions it reads each frame, turned into the focused
form's navigation keys and the player controls' command word, a camera
look and zoom, steps for the power bar and dialogue, and a rumble it
computes but never sends. Exarust had no gamepad at all. A first pass
reproduced the sets on Bevy's gamepad input; review against the listing
found the menu frame's analog bits and cursors, the pick's unit
threshold, the look rate and pitch sign, the zoom direction, the step
modes' owners and the focus stack's last pop read wrongly. Phase 8
(Polish and release: controller support), with the player's controls of
Phase 3.

## What Changes

- The three action sets and their actions (names, digital or analog, set
  membership), a default mapping to Bevy's gamepad controls, and the set
  table with its gameplay layer and focus stack.
- The menu reader: the 0.25 dead zone, the 0.1 analog bits and scaled
  cursors sent every frame, the 400/150/30 ms repeats with the original's
  fast-repeat quirk, the edge-only keys, the exit hook and the three
  millisecond priming; the directional control pick at the GUI unit.
- The gameplay reader: the camera-relative move and its run flag, the
  power-bar and dialogue step modes, the look routine at the orbit rate,
  the aim, the stepped zoom, the 500 ms power-tree hold, the command word's
  edges and held bits, the Alt and crouch key bits, the sprint flag and
  the rumble's rise, decay and motor words.
- The running game: with controller mode, the pad moves, steps, aims,
  attacks, stabs, crouches, sprints, swaps weapons, toggles combat mode and
  turns and zooms the chase camera; the keyboard and mouse stay as they
  were.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `input-bindings`: the gamepad's action sets, readers and use.

## Impact

- New `exa_game::gamepad` (and its tests); `exa_game::player` calls it
  for the steering keys, crouch, attack, stab, weapon swap, combat toggle,
  combat aim and the chase camera's look and zoom. The workspace's Bevy
  gains its `bevy_gilrs` backend; Exarust links no Steam library.
- `docs/PLAYER.md` (Gamepad), `docs/UI.md`, `docs/ARCHITECTURE.md`, the
  `exa_game` and `exa_ui` READMEs.
- Evidence: REA/Ghidra on the Steam Input functions (set and action
  resolution, the menu and gameplay frames at instruction level, the
  priming, the pick and its nested hit test, the focus push and pop, the
  layer selection, the rumble and its stub wrapper, the raw stick
  fallback), on the navigation unit's look and zoom-step routines, and on
  the player behaviour's cursor read (the aim threshold and AimSpeed's
  ease); the existing player-control and movement notes for the key word
  and the power-bar and dialogue flags. No format field changes, so there
  is no corpus step.
- Out of scope: Steam itself (the library loading, callbacks, binding
  panel, floating gamepad keyboard, Deck detection, on-screen glyphs); a
  drawn form toolkit for the menu reader and the pick to drive; the
  inventory, power, dialogue, pause and camera-mode consumers of the
  command word; the AimSpeed cursor ease; a hit caller for the rumble; the
  raw XInput fallback as a live path (modelled only).
