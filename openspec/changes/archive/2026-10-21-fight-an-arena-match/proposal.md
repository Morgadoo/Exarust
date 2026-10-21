# Proposal

## Why

The career prices, schedules and settles matches, but a match itself is
not in: which arena it is fought in, what music plays, when a round ends,
what a fallen company fighter becomes, how the next round or the return to
the hub follows, and what the company's records count. The hire list that
the career recruits from, a candidate's level and loadouts, and the arsenal
the kits go back to are not in either.

## What Changes

- The twelve arenas with their spawn spots, the indoor view amount, the
  outdoor draw flag and the steady bed flag; the variety-biased arena draw
  that never repeats the last five indoor or three outdoor arenas; the
  Beast's arena5/arena7 rule.
- The eleven match tracks, their alternatives and volumes, the
  no-repeat-of-five track draw, the Beast's own track.
- The outdoor ambience bed from rain, time of day and wind.
- Three matches a day; the return's day count on tournament days.
- Match rounds: extra rounds and lives (Elimination, Reserve), the round
  end, Continue, forfeit, the return to the hub with its payment, log
  records and the company's thirteen lifetime records.
- The match tick after its first second, who stands on each side, falls
  resolved once, dead or saved (with and without a physician), the health
  word written back and read again.
- The hire list: ten candidates, the daily renewal, a candidate's
  profession and level from the counter and the unit draw, a trainer's
  skill snap, a combatant's six loadouts.
- The arsenal: shelves by item kind, stowing and releasing flags.
- In play mode an arena level draws its track as a match start does and
  plays it at the track's volume; with an opponent the match tick runs.

## Capabilities

### New Capabilities

### Modified Capabilities

- `arena-career`: the arena draw, match music, outdoor bed, daily limit,
  rounds, tick, falls, health word, continue/forfeit/return, records, hire
  list, trainer snap, loadouts, arsenal, and the running match.

## Impact

- New `exa_game::{arena_match, hirelings, arsenal}`; `ArenaMatchPlugin` in
  `GamePlugin`; `exa_audio::LevelMusicVolume`; play mode in `exarust`.
- Docs: `docs/formats/OTHER.md` (Arena tables), `crates/exa_game/README.md`.
- Evidence: REA/Ghidra on `Exanima.exe` 0.9.5 (match start, unrepeated draw,
  arena loader, match tick, round end, fall and dead test, health write-back
  and read, continue, forfeit, return, the hire list and candidate routines,
  the trainer snap, the loadout trim, the arsenal routines, the records
  window); the track names checked against `Sound.rpk`. Notes:
  `context/re/arena-match.md`.
- Out of scope: spawning the next round's or the tournament's opponents,
  the end screen and the hub/board/manage UI, the XP award's call here (the
  award itself is in), removing an employee's kit and world character, the
  character body/kit generators a candidate calls, the outdoor environment
  randomiser, the arena save chunks.

## Phase

Phase 7, M7 "arena season": the match between the board and the return.
