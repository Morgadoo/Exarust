# Proposal

## Why

The thaumaturgy casts price and start a power, but the classes that run it
afterwards (the original's active power base and one class per power) were
missing, so no power held its focus, ticked, timed out or ended. A first
port of them landed from the decompiled code; checked against the
disassembly it released held focus from the wrong pool, lost the double
precision of several curves, ran Throw and the vanish phases on the wrong
schedule and searched the wrong power list.

## What Changes

- The base power holds its focus in the body's reserve (three quarters
  with the reserve technique), returns it to the reserve when it ends over
  two ticks, and publishes a HUD word while it persists.
- The Mind classes: Sense Minds and Mask Mind (continuous and instant),
  Confusion, Enrage, Trust, Suspicion, Loyalty, Command Dead, Control Mind
  with its possession marker, and Spirit Form with its spirit body.
- The Force classes: Shell, the bubble and barricade powers, Strikes,
  Kinesis and Throw.
- The Energy classes: Shock and Shock Weapons.
- The Displacement classes: the shared vanish phases, Vanish, the Long
  Door, Constant Displacement and the Summon.
- The unit routines: the known-power search, ending a power by id, the AI
  domain dispatch, the saved-power rebuild and Power Glean.

## Capabilities

### Modified Capabilities

- `thaumaturgy`: the active power classes behind the casts.

## Impact

- `exa_game::thaum::power` (and its `mind`, `force`, `energy`,
  `displacement` modules), `exa_game::mind` (Spirit Form's ended state),
  `exa_game::thaum::casting` (the combat factors a power sets);
  `docs/THAUMATURGY.md`, the `exa_game` README.
- Evidence: REA/Ghidra disassembly of the active power base, the Mind,
  Force, Energy and Displacement power classes and the thaumaturgy unit
  routines of `Exanima.exe` (the forces2 evidence pack), with the engine's
  vector routines read for the summation order.
- Out of scope: wiring the classes into the running game (the player has
  no power bar), the combat hit formula reading the shell and strike
  factors, the effect nodes, emissives and geometry the powers create
  (another lane), the power tree and the Mind unit routines (another lane),
  the kinesis aim sweep against the level, character generation for the
  spirit body and the demon, and the player/camera swap.

## Phase

Phase 5, visual and audio fidelity: thaumaturgy powers; the shell and
strike factors serve Phase 4's combat.
