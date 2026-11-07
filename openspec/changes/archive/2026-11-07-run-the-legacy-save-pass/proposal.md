## Why

PLAN Phase 7 (saves). The save unit's start-up legacy pass (XA_SaveGames)
was ported and tested but had no live caller: the running game never ran
it.

## Evidence

REA (Exanima.exe), checked against the assembly: the game's start-up makes
the story slot manager (player-data signature 0x6F5AEA14), runs the legacy
pass on it, then makes the arena manager (0xE8AAEA10) and runs it again,
each inside a handler that reports "Error removing legacy saves." and goes
on. The pass walks the folder's `<prefix>NNN.rsg` files (number 1 to 999),
opens each, reads the player data's signature and deletes the save when it
is below the manager's (unsigned).

## What Changes

- exa_game: `save_live::remove_legacy_saves` runs `Slots::remove_legacy`
  for the story, then the arena manager when the slot managers are made
  (with a save folder), logging each count or the failure.

## Impact

With `--save-dir`, an older-format save is deleted at start-up, before the
menu's first scan. Exarust's scan already dropped such a slot as invalid,
so the folder ends the same; the pass now runs where the original runs it.
