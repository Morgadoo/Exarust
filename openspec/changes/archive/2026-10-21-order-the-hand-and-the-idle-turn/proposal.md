# Proposal

## Why

A review of the player-controls lane against the instructions found three
faults:

- The idle turn was ordered after the hand and before the chase camera,
  while the hand runs after the chase camera: the frame's schedule had a
  cycle, and the game stops building it at its first frame.
- The pick's two limits (a prop's mobility under 0.08, the player's
  strength under 0.9 for a prop that needs strength) are double-precision
  constants compared with a widened single value. A single 0.08 or 0.9 is
  already under them; Exarust compared in single precision and let them
  through.
- In combat mode, with Ctrl held or something held, the original measures
  the reach whatever the right button does; Exarust skipped it while the
  right button was held.

## What Changes

- The idle turn runs after the hand only; the turned yaw is followed from
  the next frame. A test builds the frame's schedule of the player, cursor,
  hand, idle-turn, hotkey and player-light plugins.
- The pick's limits are compared in double precision.
- The reach's right-button gate applies outside combat mode only.

## Capabilities

### New Capabilities

### Modified Capabilities

- `player-movement`: the pick's limits in double precision; when the reach
  is measured in combat mode; the idle turn's place in the frame.

## Impact

- `exa_game::idle_orbit` (plugin order, schedule test), `exa_game::pick`
  (`classify`), `exa_game::hand` (`reach_hand`).
- Docs: `docs/PLAYER.md` (Using what the cursor is over, The reach).
  `docs/PLAN.md` is the integrator's.
- Evidence: REA (Ghidra 12.1.4) instructions of the cursor classification
  (the two COMISD against double constants 0.08 and 0.9 after CVTSS2SD), the
  per-frame request (its combat branch measures the reach before the walk
  latch is read) and Bevy's schedule build (the cycle error). No corpus.
- Out of scope: the original's walk latch, which keeps the walk to the
  cursor after the right button's release until the player stops moving
  and skips the reach meanwhile (Exarust reads the button).

## Phase

Phase 3, bodies and control: M34, the hand's drag ("Still to do:
interact").
