## Why

PLAN Phase 2 ("Rooms and portals", M2 walkable world), component
TGridEnvManager. The routine table left one branch of the per-frame room
walk unported: the forced-room branch the walk takes when the view follows
no player. Its callers were not found before. They are the arena's match
starts: a match where no entrant is the player's (Exarust's spectated
match, live since round 77) stores the room at the arena's spawn spot,
and the walk then shows only that room. With no player, Exarust's walk did
nothing, so the rooms stayed as the bench left them.

## Evidence

REA (Exanima.exe), read against the assembly:
- The walk compares the scene's control mode with the player-follow mode.
  When they differ, it takes the manager's stored room (index + 1). If that
  is 0 it returns with nothing changed. Otherwise the room is wanted, and
  the door test measures from a point at minus the largest float
  (0x7f7fc99e) on both axes. The squared distance to any door point
  overflows to infinity, and the `COMISD` reach test rejects it, so no
  door's room is added. The list, hysteresis and fade code is shared with
  the anchor branch.
- Five arena routines write the stored room: the match start's camera
  set-up when the round's fighter is not the player, the next standard
  round, the tournament rounds and the once-a-second match tick. Each
  writes the world-point room query of the arena's spawn spot plus 1.
  The spot is the arena table entry the match start copies into both the
  camera point and the player's spawn point. The save unit's level load
  writes 0.
- The world-point room query floors x/600 and z/600 cm, adds half the
  grid, and accepts the linear cell from 0 to below the cell count. This
  is `Rooms::room_at`.
- The depth pre-pass and the fade pass test the same mode, or a stored room
  that is not 0. Exarust's shells are drawn opaque black (adapted, unchanged).

## What Changes

- exa_world: `RoomTracker::update_forced` (the forced branch: the stored
  room alone, through the walk's own list and fades). It adds a
  `room_view::ForcedRoom` resource that, while active, makes the walk
  ignore the anchor and take the room at its point, or change nothing
  when there is no room there.
- exa_game: the spectate session keeps the arena's spawn spot as the forced
  point (`start_arena`) and clears it at the end. A system feeds
  `ForcedRoom` from the session. The point holds only while the match
  scene exists, because the hub load clears the stored room.

## Out of scope

- The fade pass's and depth pre-pass's other-view behaviour (Exarust has
  one view and draws shells opaque black).
- `--spectate` outside an arena has no forced point. The walk keeps its
  rooms there, as the original does when its stored room is 0.
