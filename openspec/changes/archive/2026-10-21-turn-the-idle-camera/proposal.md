# Proposal

## Why

Left alone in the original, the camera slowly turns about the player after
ten seconds: the player controls' frame watches the pointer, the player's
condition and the mouse buttons, and turns the camera's target yaw by game
time. Exarust's camera never turned by itself.

## What Changes

- The controls' idle watch each frame: idle when the pointer stayed within
  2 pixels on both axes, the condition word is 0, no foot steps, the hand
  holds nothing and no mouse button is held (a dead player is always idle).
- Past 10 000 ms of game time idle, the camera's target yaw turns by
  0.0001 rad per millisecond (double precision, then single), wrapped past
  2π, and the behaviour stops reading the cursor until the player is busy.

## Capabilities

### New Capabilities

### Modified Capabilities

- `player-movement`: the idle turn of the chase camera.

## Impact

- `exa_game::idle_orbit` (new: `IdleWatch`, `Frame`, `turned`,
  `pointer_still`, the frame's system before the chase camera);
  `exa_game::player::steer_player` reads no cursor while the camera turns;
  `exa_game::hand::HandSystems`.
- Docs: `docs/PLAYER.md` (Camera, The idle turn). `docs/PLAN.md` is the
  integrator's.
- Evidence: REA (Ghidra 12.1.4) on the controls' frame and its blocked
  branch, read in the assembly (the `< 3` pixel tests, the signed 64-bit
  stamp compare, the double 0.0001, the `ja` 2π wrap) and the behaviour
  tick's read of the cursor flag; notes in
  `context/re/player-control-hand.md` (The idle turn). No corpus data.
- Out of scope: the screens that block the controls (death screen, game
  menu) and their unconditional turn, modal windows stopping the turn, the
  dialogue and power bar, the timed removal of used consumables, the
  behaviour's open requests (inventory, loot, pickup), the gamepad flags.

## Phase

Phase 3, bodies and control: player control matching the original (the
camera, `docs/PLAYER.md`).
