# Design

## Context

The career (`exa_game::career`, `exa_game::arena`) has the boards, prices,
experience, the day and the season. The original's match runs between the
board's Begin and the return to the hub: it draws an arena and a track,
spawns the sides, ticks the match as a session hook, resolves each fall,
ends rounds, and on Continue spawns the next round or returns. The hire
list and the arsenal sit beside it.

Format and rule page: [`docs/formats/OTHER.md`](../../../docs/formats/OTHER.md)
(Arena tables).

## Goals / Non-Goals

**Goals:** every rule of the routines named in the proposal, as plain
functions with tests pinned to numbers drawn by the original's arithmetic;
the match tick wired into the running game; the arena track in play mode.

**Non-Goals:** opponent spawning, menus, save chunks, the body and kit
generators, the environment randomiser.

## Decisions

1. The engine generator is stepped in place (`roll × 0x08088405 + 1`); a
   draw under `n` is the high half of the 64-bit product. The performance
   counter's low word is byte-reversed before the first step, as the
   existing daily-board code does.
2. The recent lists and the variety count start at zero, as the
   original's uninitialised globals do, so index 0 is refused at first.
3. Compares follow the instructions: the dead tests widen to double, the
   time-of-day test is single, the round tests subtract in 64 bits.
4. The health word uses the processor's single-to-64-bit conversion
   (ties to even, out of range the minimum) and keeps the low 16 bits.
5. The loadout trim empties both hands before its weapon test, so hands
   always end empty; only the loadouts below the combatant's level are
   trimmed, since the original's loop stops at the level.
6. In play mode the arena is the one named on the command line, so the
   arena draw is not run; the track draw starts from the counter roll. The
   track keeps the volume the original passes. The match tick treats the
   player as the manager (id 0) and the only company fighter, and every
   other fighter as the other side; the team word is not modelled.
7. A stand-in counter (system time in 100 ns steps) replaces the Windows
   performance counter.

## Open questions

- What the view amount (0.3 for arena2, 0.6 for arena4) drives in the post
  processor: answered later by reading the post-process shader's use of the
  copied word (REA on the shader setup).
- What the end-screen codes 2 and 3 show: the end menu routine (REA), when
  the end menu is built.
- The records' last three words are never shown or written: confirmed by
  the window's reads (REA), nothing more to do.

## Risks

- The match tick runs whenever an `ArenaMatch` resource exists; play mode
  inserts it only on an arena level with an opponent.
