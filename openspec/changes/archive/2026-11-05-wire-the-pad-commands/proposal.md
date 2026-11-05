# Proposal

## Why

The player controls run a per-frame routine for the pad's command word,
next to the key handler. It reads the newly pressed bits and calls the
same windows and power-bar commands as the keys, with gates of its own.
Exarust read only two of those bits: `WepSwp` as the swap, and Talk,
Select and the dialogue's steps through the conversation. Powers,
UsePower, the bar's steps, PowerTree, Inventory, Skills and the pause menu
did nothing from a pad.

The player behaviour's screen-nearest character query was also kept
twice. The Talk key's target query in the dialogue code is the live one.
The behaviour module held an uncalled duplicate.

## What Changes

- Add the pad's command dispatch (`hotkeys::pad_hotkeys`) in the routine's
  order and with its gates, and run its commands through the key
  handler's dispatch:
  - Powers toggles the bar only with the player's own powers;
  - UsePower is gated by the mask only with the bar closed;
  - the steps act only with the bar open and only without UsePower;
  - PowerTree needs powers allowed and known;
  - SwapWeapon and Inventory keep the key handler's gates;
  - Skills has no gate;
  - there is no death test.
- The pad's `pause_menu` toggles the menu as F1 does.
- The scripted pad (`--pad-press`) gains `inventory`, `skills`, `swap` and
  `power-tree`. It now wins over a device the system reports as a
  gamepad: a keyboard's system-control node showed up as one and hid the
  scripted presses.
- Remove the behaviour module's uncalled screen-nearest duplicate.

Not changed: the routine toggles combat mode on the `Combat` action. The
original's pad attacks through the Steam controller layer's emulated mouse
button. Exarust's pad attacks on `Combat` (the input-bindings spec's
"attack on the combat button") and toggles combat mode with `Look`. That
choice stays as it is.

## Impact

`crates/exa_game/src/{hotkeys,menu_screen,gamepad,player_behaviour}.rs`,
`crates/exarust/src/main.rs` (usage text), the input-bindings spec.
