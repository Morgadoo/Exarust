# Proposal

## Why

The character creator drew black in the grid. A bisect between the
1 November binary and main found no first bad commit: rebuilt at the
1 November commit, and run again as the old binary itself, the creator goes
black the same way. The earlier "good" shot was taken one second before the
death fade began.

The cause is the creator's own show. The menu's screens (the creator, the
background picker, the save screens) tear the world down, with no level
loaded after it. Exarust keeps the player entity across a level change, so
the player's body went on ticking in the emptied world. About three seconds
later it was lost and heard the pit, and the death fade covered the whole
window, the creator's GUI and preview included. The original destroys the
player's body with the world. The same thing blacked out the title after a
`play` scene's EXIT (the scriptmodes lane's sandbox artefact).

Two smaller items come with this change:
- The pad's `Combat` action is the combat-mode toggle in the original's pad
  routine. Exarust read it as the attack.
- `--searcher-shots` failed in a grid quadrant: an offscreen game has no
  window to shoot.

Phase 7 (player controls and the game's screens), with the grid's debug
tooling.

## What Changes

- A plain teardown takes the surviving player's body out of the scene. It
  leaves the solver and the character ticks and is hidden, until a level
  is in again and the loading screen is gone. A body the fall took out is
  left alone.
- The pad's `Combat` bit toggles combat mode in the routine's order (after
  the power tree, before the swap), with no gate. `Look` no longer toggles
  it. The pad's attack is a stand-in for Steam's emulated left mouse
  button: the right trigger, outside the action sets, held for the attack
  request and latched on its press. `Combat`'s default control becomes R3,
  which it shares with `Look`. The mouse's left button stays the attack.
  `AltAction` stays the stab. `--pad-press` gains `attack`.
- Every debug screenshot (`--screenshot`, `--searcher-shots`) shoots the
  offscreen image in an offscreen game. The search's shots are taken one a
  frame, because a second shot of the same target in a frame was lost.

## Evidence

- Bisect logs and grid runs (`context/artifacts/2026-10-30-creatorfix/`).
- REA: the controls' per-frame pad routine and the Steam reader, traced by
  the playercontrols lane against the assembly. The Combat bit calls the
  combat-mode toggle. The behaviour's attack bit is written only by the
  mouse poll.

## Out of scope

- The original's own teardown of the player's body and the creator's build
  of a new one. Exarust keeps its surviving player.
- Steam Input: its binding panel and its mouse emulation settings.
- There are no other screenshot flags: `--figure-shots` does not exist.
