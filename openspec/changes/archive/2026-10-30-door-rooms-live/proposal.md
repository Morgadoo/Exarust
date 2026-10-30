# Proposal

## Why

Phase 2 (static world) of docs/PLAN.md, rooms and black fill. The owner
reported that the room beyond exanima06's closed golem gate is drawn: every
door counted as open, because nothing bound the placed doors to their room
links. Walking up to a shut door therefore made the room behind it active
(lit, no lid), where the original keeps it under its black lid.

Evidence (REA, Ghidra listing of `Exanima.exe`): the level object loader's
door-kind dispatch into the grid manager's door binding (twice, for the
story and arena loaders), the binding's body lookup (the object node's first
child), the door body's passage setter and the hinged and sliding steps that
call it, the room manager's per-frame active-room update, its reach and
spill callbacks and their installation, the sector classification and its
portal pass.

## What Changes

- Placed objects of the door kind are bound to their links at load with the
  body of the object node's first child, as the original's loader does.
- Each frame every bound link takes the open state of its door body (both
  directions): open without a body or while the body's pair flag is clear.
- The room-state pass makes every door neighbour of an active room adjacent
  whether its door is shut or open (it was hidden behind a shut door).
- The shut-door set is a hash set and the update no longer copies it.
- Debug: `--toggle-door FRAME` toggles the bound door nearest the player.

## Out of scope

- The navigation grid's door cells (backlog W1's routing half); the room
  exits AI reads now follow the doors.
- Callers for the reach and spill rules; drawing adjacent rooms only through
  their doorways; the lid in the lights' cube faces.
- Opening sliding doors (their lift is still behind `--sliding-doors`).

## Impact

`exa_world` (rooms, room_view, the object spawn), `exarust` (the debug flag);
docs/RENDERING.md, crates/exa_world/README.md.
