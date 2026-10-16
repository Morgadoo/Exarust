## Why

Two of the user's screenshots of the original's arena show a small red
marker on the floor: a ring at the player's front foot while it guards
against a mace (shot 2), and an arrow beside the dead player (shot 7).
Both are the original's world cursor, a sprite it draws at the pointer's
position in place of the system pointer: red in combat mode, amber outside
it, its shape following what the player does. Exarust shows the system
pointer, so every combat comparison lacks the marker, and its own aim and
walk-to-cursor point sat on a 90 cm plane where the original's lies at
80 cm.

Phase 4 (Combat and items), milestone M4 "first duel": the minimal
interface of a duel. It is item L4 of the combat-look plan.

## What Changes

- The world cursor, drawn in play mode at the pointer's position from the
  installation's `cursors.rfi`: frames cut from its 2 × 4 atlas (ring, one
  and two chevrons, arrowhead, curved cross, crosshair), resampled to
  round(3 × GUI scale) × 2 physical pixels (30 at scale 5), over a
  four-copy black outline, red (255, 76, 76, alpha 180) in combat mode and
  amber (255, 205, 76) outside it. The ring and chevrons turn away from the
  player; the arrowhead tilts 33° with its tip on the pointer. The system
  pointer is hidden only while the world cursor is drawn, and never moved.
- The shape as the original picks it: in combat mode the ring (also down
  or collapsing), chevrons while walking to the cursor (two beyond 300
  cm), the arrowhead with Ctrl and after death; outside it chevrons while
  walking to the cursor, else the arrowhead over ground within 3 m and the
  cross beyond, and the cross while down or dead.
- The original's GUI scale from the window's physical height (3, 4, 5, 6,
  8 or 12 from 700, 870, 1051, 1390 and 2080 pixels), as a shared
  resource for the interface.
- The cursor's point (walking to the cursor, combat facing) on the 80 cm
  aim plane, and found through the scene camera's viewport: with
  supersampling the camera draws into an image twice the window's size,
  and the window position used to land up and left of the pointer.
- Debug helpers for unattended checks: `--cursor x,y` holds the pointer at
  a window position; `--press` takes `LMB`, `RMB` and `Ctrl`, and
  `key@a-b` holds a key or button from frame a to b.
- Docs: UI.md (Interface scale, World cursor), PLAYER.md, DEBUGGING.md,
  PLAN.md (Phase 4), the exa_ui and exa_game READMEs.

Evidence:
- Reverse engineering of `Exanima.exe` 0.9.5 (REA, Ghidra, the
  disassembly): the cursor's load and draw routines (size formula, frame
  offsets, the pointer's turn and shift, outline, colours and the red's
  alpha latch read from the image), the GUI scale routine and the window
  size handler that picks it, the combat branch of the player behaviour
  tick and the exploration movement request (the shapes), the reach test
  under the pointer, the texture filter flags (linear, no mipmaps), the
  80 cm aim constant and the option flag behind the pin and warp
  (`context/re/corpses-and-markers.md` §3, `combat-look-plan-final.md`).
- Corpus: `cursors.rfi` (256 × 512 A8; a test of the installed atlas).
- Live retail, read-only: the cursor modes logged over the user's fights
  (ring in combat, also down and collapsing; chevron on the right button;
  arrowhead after three deaths in combat mode; arrowhead or cross between
  fights), the GUI scale 5 and the 1600 × 1014 client area, and a
  screenshot of the amber arrowhead beside the player between fights.
- Exarust in game through `gpu_run.sh`, with screenshots of every shape.

Out of scope:
- the option that pins the cursor to the 80 cm plane and warps the system
  pointer (off by default in the original);
- walking to the cursor with the right button in combat mode (the
  original does; Exarust's footwork does not yet), so combat chevrons wait
  for it;
- the collapse (L5: the shape rule already covers it), interaction targets
  (the crosshair, Ctrl outside combat), GUI states that hide the cursor or
  force the cross, and the menu cursor (`menucursor.rfi`);
- the health bars (L14), which will use the GUI scale.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `user-interface`: the world cursor, its shapes, and the interface scale.
- `player-movement`: the cursor's point on the 80 cm plane, through the
  camera's viewport.
- `world-viewer`: scripted mouse buttons, holds and pointer position.

## Impact

- `exa_ui` (`scale.rs`, `cursor.rs`, `lib.rs`)
- `exa_game` (`cursor.rs`, `player.rs`: the aim plane, the viewport
  conversion, shared constants; `lib.rs`)
- `exarust` (`main.rs`, `viewer.rs`: `--cursor`, `--press` holds and
  buttons)
- `docs/UI.md`, `docs/PLAYER.md`, `docs/DEBUGGING.md`, `docs/PLAN.md`,
  `docs/COMPONENT-MAP.md`, `crates/exa_ui/README.md`,
  `crates/exa_game/README.md`
