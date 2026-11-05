# Player controls and player behaviour: routine map

Two player components of the original engine, audited routine by routine on
2026-11-05 (playerctl lane). Routines are named by role; the research notes
and the lane's artifacts keep the retail addresses.

## SG_PlayerControls (unit routines)

123 routines, about 49.9 KB (one is a `TPlayerControl` method). 78 have a
role in the research notes (input layer, drag and lift handlers, item use,
interaction, key and pad handlers, the controls' frame, the player light,
the life cycle) and were mapped by earlier lanes. 45 small helpers (about
10 KB) have no note and were not audited.

The spectate session (this lane):

| routine | bytes | before | after | evidence |
|---|---|---|---|---|
| Spectate start hook | 368 | missing | live behind `--spectate` | sets the mode, turns the controls on, sets the spectate flag, clears the camera's follow, registers the double click's interaction routine |
| Spectate end hook | 208 | missing | ported (session end), no caller in play | turns the controls off, clears the flag and the follow, drops the click |
| Interaction (double click), spectate branch | part of 1,504 | missing | live behind `--spectate` | a picked character's body becomes the camera's follow object; anything else, or a miss, frees the camera |
| Controls' frame, spectate gate | (already ported) | live (gate never set) | live (the session sets it: the player is not steered) | the frame returns while spectating |

Reachability: the mode's hooks are called only from the arena's round entry
(the next round's entrant, the tournament round and two other entries): an
entrant that is the player starts the Character mode, any other (a company
employee) starts Spectate. Exarust's arena always fields the player, so play
never enters the session; `--spectate` enters it to check it.

## TPlayerBehaviour

25 routines, about 17.9 KB. The gap note's `closest_to_cursor` is the
screen-nearest character routine: its callers are the controls' key
handler (Talk) and pad routine (Talk), both in SG_PlayerControls, and it is
live as `exa_game::dialogue_live::talk_target`; the behaviour's own copy was
an uncalled duplicate, removed in round 62. Of the 25, 18 are live or
mapped, 2 are runtime glue (destroy, the mouse capture release on disable)
and 4 small or variant routines (about 1.6 KB: a helper, the head tracker
with another eye offset, the disable hook that leaves the party) were not
audited. The behaviour tick now leaves the player unsteered while the
controls spectate.
