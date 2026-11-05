## Why

Phase 2 (player and controls), SG_PlayerControls and TPlayerBehaviour, the
playerctl gap lane (round 75): the arena spectate session was the controls'
named gap, and TPlayerBehaviour's note said `closest_to_cursor` had no
caller.

## Evidence

REA: the Spectate mode's start and end hooks (flag, controls, camera
follow, the double click's registration), the interaction routine's
spectate branch (the picked character's body becomes the follow object),
the set-mode routine, and the hooks' only callers in the arena's round
entry (an employee entrant). The screen-nearest character routine's callers
(the key handler's and pad routine's Talk).

## What Changes

- `exa_game::spectate`: the session (start, end, the double click's follow
  rule), the chase camera's focus while spectating, the behaviour tick's
  steering stopped while spectating; `--spectate` starts it after the load.
- docs/re/playerctl-map.md: both components' routine maps.

## Out of scope

- The arena's employee entrants (XA_Arena), so play never spectates yet.
- The interaction routine's item branch while spectating; the frame light
  at the view's focus.
