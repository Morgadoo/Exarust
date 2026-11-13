## Why

Roadmap task B0.5, found by the modes-03 audit. A new story or career never
made a save slot, so the story's exit, revive, level-exit and checkpoint
saves wrote nothing until a slot was continued from the saves screen. The
flow's save steps ran only in story mode, so the arena's saves (the board's
Begin, the quit, the forfeit) were never written. The golem alarm, which the
level section's settings chunk carries, was never saved or restored, and the
journal chunk was written from a fresh journal and the loaded journal was
never shown.

## Evidence

REA (Exanima.exe), checked against the assembly:

- The session begin's callback calls the empty-slot creator on the arena
  manager for game mode 8 before the career start, and on the story manager
  for mode 4 before the story start (keeping the start background word
  across the call); practice calls neither.
- The save writer and the forfeit's save write the current manager. The
  board's Begin and the auto rematch's timer callback call the save writer
  (no death rule) before their launch.
- The quit hook (installed at the begin, called by the window's close)
  saves the story with the death rule; in the arena it reads the menu
  form's page: page 2 saves, page 4 runs the forfeit, which settles the
  match and, unless the manager has fallen, writes the player data alone
  with the locale `arenahub`.
- The settings chunk's four globals are set by the golem alarm trigger
  (gate byte, pass word, twelve-byte point, count plus one); the settings
  hook reads the point's height against `1 000 000.0` back into the gate.

## What Changes

- `flow::Step::NewSlot` at a story's or career's begin; `SaveStep::NewSlot`
  creates the slot in the mode's manager under the created character's name.
- `run_save_steps` writes the mode's manager: the arena's save with its
  tables and the manager, and `SaveStep::ForfeitSave` (player data only).
- The board's Begin and the auto rematch ask for the save before the launch,
  ordered before the level change's teardown.
- `save_live::QuitAutosave`: the app's close runs `Flow::quit_session` in
  `Last`, with the frame's save, screen-save and forfeit systems mirrored
  there (each reads every frame; only the one for the app's state acts).
- `GolemAlarm::saved_settings` / `from_saved_section` and
  `golem_alarm::restore_saved_alarm`.
- Saves write `GameScreens::journal`; a story load puts its pages back.
