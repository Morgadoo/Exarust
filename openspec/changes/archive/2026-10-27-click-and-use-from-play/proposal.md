# Proposal

## Phase

Phase 3 (Bodies: skeleton, physics, control), the player-control lane
(`SG_PlayerControls`), with the HUD show rule of Phase 7.

## Why

The player controls' click, double-click interaction, item uses, timed
list, request modes, hand drag states and mechanism pulls were tested
functions written from the pseudo-code; several read the wrong byte or
branch, and the running game did nothing on a click on the player's own
body.

## What Changes

- The click's outcomes follow the click handler's branches: no usable
  section shows the item, kinds 0 and 1 open the container, the 1000
  reward is a lock subtype 3, and the own body shows the HUD bars for
  5000 ms (wired: `exa_ui::hud::HudShowUntil`).
- The double-click interaction: only the hand-held item ends it; every
  other branch goes on to the container; it runs only on a double click,
  in combat mode with Ctrl held. The double-click time is the system's
  (500 ms by default).
- Item uses: the elixir's sound only with the potion; a repair marks
  0xFF0000 and plays at the player; the pitch sum in double; a container
  mark by slot.
- The timed list skips the weapon section the frame the usable section is
  not written; the request mode tests only use, lift, door, refusal and
  combat.
- The hand: the drag state word, release order, door, wheel and cloth
  pulls, the mechanism press and the box test in the full affine frame;
  the wheel's wrap compares in double.

## Evidence

The click handler, the double-click handler and its object helper, the
use routines and mark helper, the timed list, the per-frame request, the
power hold and its reader, the box test and its point transform, the door
and wheel drags, read in the assembly (`context/re/player-control-hand.md`,
`context/re/hud.md`); the double-click time is `GetDoubleClickTime`.

## Out of Scope

The item cursor, the loot, container and dialogue windows, the scene
script registry, live door, wheel and cloth bodies, the GUI item drag and
the item factory spawn.
