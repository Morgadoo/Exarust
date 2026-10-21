# Proposal

## Why

The original's game-main unit owns the game's top-level flow: the start-up
into the title, the mode switch, the story, practice and arena session
starters, the per-frame session hooks (the story's wake-up, knock-out and
revival, the practice round), death and the end-of-session menu, level
exits with their save and checkpoint, the respawn, the application quit's
autosave and the story scripts' `GameEvent`. Its start-up splash shows the
two BME logos before the main menu. Exarust had the menu pages, the
practice ladder's numbering, the arena hub and the save packages, but not
the flow that orders them, nor the splash.

Phase 7 (Game modes and content), the game-mode flow milestone; the splash
is Phase 8's start-up polish but rides with it.

## What Changes

- `exa_game::flow`: the mode and phase words, the mode switch, the story
  start (spawn table, the engine's angle builder, starting light, Derrin),
  the practice start and next fight, the story and arena continues, the
  camera shots and cursor warps, the transition's 100 ms actions, the frame
  task (dialogue then hook), the story hook, knock-out and revival, death
  and the end menus, the level-exit save rule, the respawn at `exanima05`,
  the application quit's autosave, the exit-trigger nudge, the practice
  scene filter, the music choices, the unlock words and the save managers.
  The rules return named steps; `FlowPlugin` runs the flow on the game
  clock and logs the steps; F1 opens the flow's page.
- `exa_game::game_event`: the story scripts' memory fragments, raise-dead
  lines, Corlian's line, the three domain awakenings and the background
  unlock.
- `exa_game::splash`: the splash's state machine on the original's two
  clocks, its logo placement, grey-level fades and backdrop alpha, and its
  drawing; `exarust --splash` shows it at start-up and opens the main menu
  at the hand-over.

## Out of scope

- Applying the world-changing steps (level loads, teardown, placing bodies,
  camera moves, saves and checkpoints, the end menu and tips): the flow
  only logs them.
- The transition screen's drawing, the prelude, the help screens, the
  splash's sound and the menu theme's playback, the audio fades.
- Scripted modes' hook tables and the versus lobby; the arena's own end
  menu and pause callback.
- Showing the splash by default: Exarust's launcher opens a view of its
  own, and the splash would cover its first five seconds.

## Evidence

REA (Ghidra 12.1.4) on the game-main unit and the splash screen's routines
(the gamemain pack's pseudo-code, assembly and constants), read against
their assembly; the engine's vector add, subtract, squared distance,
vector-by-matrix and point-to-box helpers, its angle builder and x87 sine
and cosine, the spawn and identity tables and the backdrop colour read from
the executable. Real data: the installed logos' sizes (`EXANIMA_DIR`) and a
GPU run of `exarust --splash`.
