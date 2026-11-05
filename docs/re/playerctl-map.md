# Player controls and player behaviour: routine map

Two player components of the original engine, audited routine by routine on
2026-11-05 (playerctl lane). Routines are named by role; the research notes
and the lane's artifacts keep the retail addresses.

## SG_PlayerControls (unit routines)

123 routines, about 49.9 KB (one is a `TPlayerControl` method). 78 have a
role in the research notes (input layer, drag and lift handlers, item use,
interaction, key and pad handlers, the controls' frame, the player light,
the life cycle) and were mapped by earlier lanes. The other 45 (about
10 KB) were audited in round 77:

| group | routines | bytes | status |
|---|---|---|---|
| the lift onto the cursor (frame placement, turn with the 300 ms ramp, placement at a point, per-frame update, the least distance over linked parts, the 280/−1000 cm box test, the lifted-state test and restore, the flag pass) | 11 | 4,272 | live (`hand`, `item_cursor`) |
| drag events (release into a container or the world, the GUI drop, the release event, the 0xFB01/0xFB02 hook, the nearby-object notify within 5 cm) | 5 | 1,296 | live (`hand`, `item_container`, `player_controls`) |
| item use (clean, repair, key into a lock with its sound, the used item's marks and consumption, the use-anim list's pruning, a sound at a point, an object spawned by id) | 8 | 1,632 | live (`use_cursor`) |
| cursor look and reference point (the player's; the spectating camera's branches are not run by Exarust's chase camera) | 3 | 1,088 | live (`player`) |
| the controls' scene start (night-vision light, deferred callback), the hotkey slot due in 500 ms, key word to scan code | 3 | 304 | live (`player_light`, `hotkeys`, `bindings`) |
| scene-graph and runtime glue (character list, node and frame refreshes, basis orthonormalisation, flag clears, mouse capture, state getters, six string finalisers) | 15 | 1,392 | glue (Bevy/ECS) |

None of the 45 is reachable and missing.

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
