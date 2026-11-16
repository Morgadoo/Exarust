## Why

Roadmap task B4 (lane `motileai4`): TMotileAI's open ledger rows. The room
bookkeeping linked every brain into a 600 cm cell vector that the original
does not keep (its lists are per level room), the nearby gather read every
fighter standing in a room, and the player's contacts read every fighter
in the level; a despawned searcher kept its object reserved; the death
hook's golem branch and the chase leave's target clear were missing.

## Evidence

REA (Exanima.exe 0.9.5), read in the instructions:

- The room move: the body position times 1/600, floored, plus half the
  grid's width and height (unsigned halves), index `width·z + x` in 32
  bits; a negative index or one not below `width·height` returns at once;
  the cell-to-room word less one is the room; a negative room calls the
  unlink (nearby count 0, list unlink, room −1); the same room returns; else
  the body is unlinked from its old room's doubly linked list, pushed at the
  head of the new room's, the old room (when one) becomes the previous room,
  the room is stored and the room walk's exit mask is cleared. The AI tick
  calls it before its condition gate, behind the scene's grid-mode flag; the
  player's behaviour calls it after its contacts, past its stagger and
  suspend exits.
- The nearby gather: only in a room with a room table; the count is
  cleared, the room's list is walked skipping the brain's own body, then the
  room behind each exit (the exit's cell's room word less one, a repeat of
  the previous exit's room skipped, starting from −1) with no self skip,
  each through the insert. The AI calls it from the contact integration's
  present branch; the player's behaviour calls it first in its tick and its
  contacts read the nearby list (count and entries).
- The death hook: record race word & 0x1000 calls the material fade with
  the `go` word and 0.0 before the follower branch.
- The chase's leave: the body's target is remembered, then (unless the
  byte at the chase's hold) the body's target and focus words are cleared
  before the idle return.
- The behaviour skill a foe's estimate reads is written only by the AI's
  constructor (combat skill / 100, capped at 1) and the companion
  conversion; the player's behaviour constructor and the shared base never
  write it and the instance starts zeroed, so a player foe's skill is 0.
- The removal hook calls the idle activity's slot 14: the search's stop,
  which releases its object reservation.

## What Changes

- `exa_ai::rooms::RoomLists` replaces the 600 cm cell vector: per level
  room, newest first; `move_to`, `unlink`, `gather`, `grid_cell`;
  `RoomPlace` for the player. The AI tick moves rooms before its condition
  gate and gathers in its present branch; a brainless body is linked by the
  AI tick [Exarust].
- The player's behaviour gathers its nearby list, builds its contacts over
  it and moves rooms after them.
- The search's removal hook (`release_removed_searches`).
- The death hook's glow fade (logged; no live skin material carries `go`).
- The chase leave clears the combatant's target and the duellist's focus.

## Impact

`exa_ai` (brain, rooms, lib), `exa_game` (player_practice, player,
npc_search, flow). Specs: npc-perception, npc-idle, player-movement,
npc-search, npc-attack.
