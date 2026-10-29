# Proposal

## Why

docs/PLAN.md, Phase 6 (the power tree screen, thaumtree lane). On the GPU,
`exarust --powers 0x201 --press P@200 --screenshot … play arena` logged the
power tree hotkey but the screenshot showed neither the tree nor the domain
wheel. The debug simulation controls also read `P`: the same press paused
Bevy's virtual time, the clock the tree's opening fade runs on, so the
fade stayed at its first frame (window alpha 0) and the opened screen drew
nothing visible. The game itself was frozen too.

## What Changes

- The simulation pause key moves from `P` to F8.
- No simulation key (pause, step, slower, faster) fires while it is bound
  to a game action in the player's bindings; a rebinding onto one of them
  goes to the game.
- A test presses the tree's bound key the way `--press` does (after the
  input systems, released the next frame) with the simulation controls
  running, and checks that the game clock runs, the screen opens and fades
  in, its canvas is shown, and with the installed packs the backdrop covers
  the window and the wheel is lit.
- docs/DEBUGGING.md, docs/PLAYER.md, docs/THAUMATURGY.md and the viewer's
  usage text name F8.

## Evidence

Reading the input path: `--press` writes `ButtonInput<KeyCode>` after the
input systems; both the hotkey handler and the screen saw the press. A
MinimalPlugins app with the simulation controls reproduces the bug (the
virtual clock is paused after the press) and passes with the fix; the same
app with the installed packs renders the open tree, bar and wheel.

## Out of scope

The tree canvas's z-order against the HUD layers and its premultiplied
output (unchanged).
