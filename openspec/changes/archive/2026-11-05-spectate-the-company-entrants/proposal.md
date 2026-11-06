## Why

Phase 5 (arena), XA_Arena and SG_PlayerControls, the playerctl lane (round
77): the controls' Spectate mode was ported behind `--spectate` because
Exarust's arena always fielded the player. The original enters it from the
arena's match start and round entry whenever no participant is the
player's to control, which the board's default lineup does as soon as the
company has a hired combatant.

## Evidence

REA: the match start (the player record cleared, the scene torn down, the
loadout pass naming the first controllable participant the player, then
either Spectate with the camera at the spawn spot raised by
(700, 1180, 1100) cm or Character mode with the player body built from the
record), the next round's entrant routine (the same choice for the
entrant that comes in), the tournament spawn and the match tick (the
mode's other callers), the Duel template's placement (an entrant is placed
unless it is the player), the Spectate end hook (no direct caller: the mode
table's end hook, run when the next mode starts).

## What Changes

- `arena_hub_live`: the launch carries the lineup's pool slots
  (`entrants`); `--hub-enter` begins the board's match with a given lineup.
- `arena_live`: company entries are mapped through the lineup (name,
  health, participant id; the manager from slot 0); the first round waits
  for the bench; a spectated match's fighters duel the nearest other side.
- `save_live`: the bench (the manager's record kept as the next player
  spawn's, the player despawned, the session started).
- `spectate`: `start_arena`, the arena mark, and the end at the hub.
- `scene_bodies`: bodies spawn without a player (no rating player needed).

## Out of scope

- Recruits (class 1, controllable): Exarust's roster has no recruit yet, so
  only the manager can be the player's entry.
- The original's fixed camera position (Exarust's chase camera orbits the
  spawn spot).
- A reserve match whose manager enters in a later round.
