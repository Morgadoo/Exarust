# Proposal

## Why

docs/PLAN.md, Phase 3 (bodies: skeleton, physics, control), the player's
controls: the playerctl4 lane (SG_PlayerControls unit routines) added the
placement's sweep to first contact, a hover word, the GUI drag's spawned
record and object spawner, the use release's dispatch, the door drag's stop
frame and the game-mode registration. Its review checked each rule against
the executable's instructions and corrected several, one of them in the
running game: every cancelled lift over the level stayed where it was
instead of going back.

## What Changes

- Placement sweep: from 600 cm before the picked point along the ray (the
  original backs the ray's point off 600 and searches 1000 on), never from
  the camera; a sweep starting inside the level is no contact; the
  nearer of contact and pick is kept.
- Hover word: 0x10 for a lift snapped onto a level placement marker of its
  class, 0x30 when the marker's link byte reads `L`, 1 for a free placement;
  the live hand leaves it at zero (no markers), so a cancelled lift is put
  back. The invented inventory-slot hover is removed.
- GUI drag record: both drag events spawn the dragged entry with the drag's
  condition.
- Use release: a door unlocks or rattles only when locked; a world item
  needs an item id; a character view takes a liquid; an item window is the
  GUI use.
- Game modes: Character then Spectate, each id the table's new length plus
  two.

## Evidence

The placement sweep, its per-node recursion and the placement routine, the
lift's frame (marker snap and hover word), the GUI drag's spawn and
release, the use dispatcher and its liquid and kit routines, the door drag
frame, and the unit initialisation, read from the disassembly.

## Out of scope

A sweep of every collision sphere against the level meshes and the
original's support-plane offset per sphere; level placement markers; an
inventory take for a world body; driven door, wheel and cloth bodies; a
live usable-section reader for the right-press use.
