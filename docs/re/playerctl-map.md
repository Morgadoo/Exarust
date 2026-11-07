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
session with the camera at the original's fixed pose (the spawn spot
raised by 700, 1180, 1100 cm, looking at it); every match fighter's brain
picks its foe through its target pass (round 80); the hub's arrival rebuilds the manager and ends the session.
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

Round 81, the method table (FPC VMT slots from 96 bytes on, the class map's
scan):
- The head tracker with the other eye offset is slot 34 of the combatant
  class, inherited unchanged by every behaviour (AI, player, character
  view). Nothing calls through that slot on a behaviour: the only indirect
  calls at its offset are on GUI and other classes' objects. It never runs.
  (Slot 33, the head tracker, runs: the player behaviour's slot 33 calls it
  directly, the AI tick through the slot.)
- The "disable hook that leaves the party" is not the player behaviour's:
  it is the AI class's slot 25. Slot 25 is the death hook, called through
  the behaviour by the body's death routine. The player behaviour's own
  slot 25 is the mouse-capture release (glue). The AI's hook ends a
  follower's follow and stops a record-flag effect; it belongs to the AI
  component.

The behaviour tick (4,304 B) read whole against Exarust: the third-tick
snap, contacts, activation centre, practice lesson, key poll, sprint latch,
held-wake release, exploration steering and the combat ladder, step length,
soft stop, cursor modes, CamFollow, auto-target, the attack executor and
recovery blends are ported. One branch was not: while the narrator's reply
panel offers choices, any player input (buttons over the game view, a
movement key, the stick) sets the dialogue unit's end flag, which ends the
narrator's conversations; now `dialogue_live::narration_input`. The
cast/use callback the ladder calls with 1, 2 or 8 (the cast and use paths'
own hooks) is left to those paths.

## The playerctl2 lane (after round 123): the mouse capture, the cast hooks, the AI's space test

The orbit and Cursor Look are one routine of the controls' frame, with
three helpers. Exarust had the capture's lock, the turn and a warp back to
the saved point.

| routine | bytes | before | after | evidence |
|---|---|---|---|---|
| Mouse capture (orbit and Cursor Look) | 1,310 | partial: Cursor Look only, the pointer centred once, the body's yaw turned, the warp always to the saved point; the orbit read the raw button with no capture | live (`player::cursor_look`): one capture for both keys, its kind fixed at the start; the pointer re-centred every held frame; Cursor Look turns the cursor follower's target (the behaviour turns the body) and aligns the camera behind the drawn cursor direction every frame; the orbit drags from the next frame; the GUI pointer is the aim point's screen point; the release sends the drag's close message for both kinds and warps to the aim point's screen point (the saved one when it began over the GUI); spectating, the look's direction and distance from the camera's look-at point | unit tests (player, steering) |
| Aim point's screen point | 381 | missing | live (`player::aim_point`, `window_point`): the standing point at the ground plus 80 cm, along the drawn cursor direction at the cursor's last distance; spectating, from the camera's look-at point | unit test |
| End-of-frame view snapshot | 143 | missing | live (`CursorLook::last`): the view and the reference point (the player's feet, or the look-at point) | unit test |
| Pointer pin while the camera aligns | 855 | missing | live: neither key held and the camera still aligning (the align routine's flag, cleared within 0.01 rad, compared in double, or by a drag), the pointer's point on the aim plane under the frame before's view keeps its flat offset from the player | unit tests (`ChaseCamera::align`) |
| Pad aim's screen cursor reset, pad aim warp | 65 + 72 | n/a | n/a: Exarust's pad aim eases a world point that starts from the mouse's point each time a stick asks (`player::pad_aim_cursor`) | — |

The behaviour tick's side (its cursor routine, 1,515 B, already live): while
the controls hold the mouse the cursor's point is not read and its distance
is kept; while Cursor Look holds, the raw cursor direction is the look's
target, written after the follower's tick (`steering::Capture`).

The capture's return value (whether the frame's mouse was taken) is not
used by Exarust's frame. The pad's look keeps its own branch (it ends a
capture). A cursor follower reset by a down body while Cursor Look holds
takes the look's target again on the next tick (the original's next turn
starts from the heading); Exarust keeps the look's own target.

TPlayerBehaviour: the ladder's callback is the caster body's cast hook,
which only the force powers install (blast, wave, torrent, barrier,
bubble, barricade, shell, kinesis, throw, strikes). The behaviour tick asks
it with 1 while the left button is down (after the key poll), with 8 each
tick outside combat mode, and in the casting condition with 2 (the right
button, or the pad's sprinted move) else 1 (the left); a negative answer
ends the cast. It is ported as `thaum::casting::player_hooks`, live from
the casters' tick (`thaum::cast`). One gate differs: the original's tick
returns before the asks while the body's shock-stagger state flag (0x100)
or the behaviour's own disable byte is set, and Exarust asks unless the
body is dying; left for the thaumaturgy lanes.

TMotileAI: the "is the space free" query has one AI caller, the
behaviour's method slot 26, which only the story revive's world pass calls
(over every scene character; the player's slot is empty). It is the reset
`exa_ai::presence::reenter` already modelled as a level-leave reset (that
premise was wrong): healed, contacts cleared, and a roaming AI moved along
two rounds of wander and home-side plans, each planned from where the last
ended; then placed, asked the space query, and moved to the grid's nearest
walkable point when it meets another body; it leaves combat, stops
following and returns to idle. Now live (`exa_game::revive_pass`, the
flow's `ReviveBrains` step after the alarm return). No AI routine asks the
query to place items, choose a stand spot or step.
