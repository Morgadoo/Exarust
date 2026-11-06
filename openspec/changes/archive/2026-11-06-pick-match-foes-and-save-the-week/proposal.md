## Why

Round 80 playerctl lane: replace the spectated fighters' nearest-foe
stand-in with the brain's own target pass, put the spectate camera at the
original's pose, and save the week's events and the hire list.

## Evidence

REA: the target pass's combat entry (enter combat, then set target to the
pick); the match setup's AI timers; the spectated match start's camera
offset (700, 1180, 1100) cm; the tournament chunk's writer and loader (per
event the template id, the event index, the weapon row and the word after
it; the loader copies the template and sets the rank window); the hire
list's writer and loader (an unused word and the v7 character record).

## What Changes

- exa_ai: the pass's entry sets the duellist's foe to the pick; a held AI
  clock (`Duellist::hold_until_ms`) defers the entry.
- exa_game: match fighters get Npc brains with relations and the grace
  hold; the stand-in is removed; the spectate session's fixed pose;
  `arena_tables` writes and reads the hire list and the week's events.

## Out of scope

- The season's week verdicts in the tournament chunk (kept from the save).
