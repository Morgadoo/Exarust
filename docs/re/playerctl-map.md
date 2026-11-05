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
| Spectate start hook | 368 | missing | live (a match with no controllable participant; `--spectate` too) | sets the mode, turns the controls on, sets the spectate flag, clears the camera's follow, registers the double click's interaction routine |
| Spectate end hook | 208 | missing | live (the hub's arrival, as the next mode starts) | turns the controls off, clears the flag and the follow, drops the click |
| Interaction (double click), spectate branch | part of 1,504 | missing | live while spectating | a picked character's body becomes the camera's follow object; anything else, or a miss, frees the camera |
| Controls' frame, spectate gate | (already ported) | live (gate never set) | live (the session sets it: the player is not steered) | the frame returns while spectating |

Reachability: the start hook is called only from the arena's match start,
the next round's entrant, the tournament spawn and the match tick. The
match start clears the player record and tears the scene (and its player
body) down; the loadout pass names the first participant whose record the
player may control (the manager, or a recruit) the player. With one, the
Character mode starts and the player body is built from that record; with
none (only hired employees entered, which the board's default lineup does
once a combatant is hired) no player body is built, the camera is put at
the spawn spot raised by (700, 1180, 1100) cm looking at the spot, and
Spectate starts. The end hook has no direct caller: it is the mode's end
entry, run when the next mode (the hub's Character mode) starts.

Exarust (round 77): the board's lineup travels on the launch; a match with
no player entry benches the manager (his record, possessions and health
kept for the next player spawn, the player despawned) and starts the
session looking at the spawn spot; the fighters duel the nearest other side
(Exarust's stand-in for the brain's target pass, which otherwise follows
the player); the hub's arrival rebuilds the manager and ends the session.
Recruits are not in Exarust's roster yet, so only the manager can be the
player's entry.

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
