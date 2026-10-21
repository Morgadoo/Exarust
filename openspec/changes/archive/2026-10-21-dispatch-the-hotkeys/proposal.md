# Proposal

## Why

Exarust read Tab and R directly, whatever the player's state: a dead
player could still toggle combat mode, the swap ignored the body's busy
bits, and F did nothing. The original's key handler decides every hotkey
in one place, by the player's state, the open dialogue or power bar and a
permission mask, and its combat toggle lets go of the hand and clears
request bits.

## What Changes

- The key handler's decision for every hotkey (dead, power bar, dialogue,
  then the play keys in their order with their permission and power
  conditions).
- Wired: Tab (not while dead; entering combat lets go of the hand and
  clears the backpedal, keys' heading and turn-about bits), R (refused
  while the condition word has 0x10202 or a lift is on the cursor), F (the
  record's light flag of the first held item).
- The binding slots 18–20 named by what the key handler does with them
  (power bar, power tree, use the picked power), and a Bevy key's virtual
  key code.

## Capabilities

### New Capabilities

### Modified Capabilities

- `player-movement`: the hotkeys and the combat toggle.

## Impact

- `exa_game::hotkeys` (new), `exa_game::bindings` (`virtual_key`, the power
  slots' names), `exa_game::player` (the old Tab and R systems removed).
- Docs: `docs/PLAYER.md` (Bindings, Hotkeys). `docs/PLAN.md` is the
  integrator's.
- Evidence: REA (Ghidra 12.1.4) on the key handler, the combat toggle, the
  swap request and the light toggle; notes in
  `context/re/player-control-hand.md` (Hotkeys). No corpus data.
- Out of scope: the windows, the dialogue and the power bar the other keys
  open; the gamepad's hotkey flags; the extinguished light's effect on the
  held torch; the original's lift cancel on entering combat (it restores a
  saved pose and leaves a drag's link); the flag that keeps a player out of
  combat mode.

## Phase

Phase 3, bodies and control: player control matching the original
("Still to do: interact"; the bindings' power slots).
