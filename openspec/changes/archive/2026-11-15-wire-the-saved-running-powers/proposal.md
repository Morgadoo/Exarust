## Why

Roadmap task B1 (wiring lane `wiresave`, docs/PLAN.md Phase 8 "saves and
the session"), findings F187, F204, F150 and F287, F417 and F121 of the
powers, items, modes and UI audits. A character's running persistent
powers were neither written by the live save nor rebuilt by the load: the
save and restore routines of the power classes, the Mind and Force
restore rows and the record build's restore walk had test callers only.
The icon cache was never cleared on a load, the save unit's GUI reset had
no live caller, the crystal drain's sources accumulated across levels, the
adapter journey's save was asked for and never written, and the slot
delete did not log its line.

## Evidence

REA (Exanima.exe), checked against the assembly where constants or order
matter:

- The exporter's placement writer, after the motion state in save mode,
  walks the body's power list when it holds a power: a running-powers
  sub-chunk with a count word, each power's save slot, the count patched
  with the powers that answered, the chunk dropped when none did.
  Its only callers are the story and arena player-data writers; the level
  export's character walk writes the flat pose, pose tree, AI home and
  motion state but no running powers.
- Six classes have a save slot: the persistent Sense Minds and Mask Mind,
  Shell and Strikes write their id and 0; Loyalty and Command Dead their
  id, 4 and the target's character id.
- The record build (the character builder from a placement) hands the
  chunk to the restore walk: count, then id, size and bytes; Mind and
  Force by the id's domain byte, each through its table row's restore
  constructor; the row's cost is stored as the held focus and added to
  the reserve (three quarters with the reserve technique); the walk then
  multiplies the held focus by 0.75 for a body with the technique.
- The restore constructors leave the self powers running (state 0); the
  Loyalty and Command Dead constructors call their load slot (the target
  word) and set the ready state; their ready tick looks the target up by
  character id, adds the marker with the caster and re-arms the target's
  behaviour.
- The player spawn and the content stream's character fetch run every
  power's activation slot after the build.
- The save unit's GUI reset calls the death fade reset, the sweep of every
  game screen, the crystal source clear and the icon cache clear, then
  zeroes a game-main byte.
- The selection screen's travel step calls the save unit's save then copy
  over the checkpoint; the slot delete logs `Save delete index: ` and the
  slot's number.

## What Changes

- `exa_formats::characters`: `RunningPower`, `read_running_powers`,
  `write_running_powers`, `Placement::powers` (read from the placement's
  running-powers sub-chunk); `save_package::CharacterPlacement::running_powers`.
- `exa_game::thaum::power`: `saves_running`; `restore_persistent` takes
  the saved records, keeps the Loyalty and Command Dead target word and
  holds what the reserve took (the Force held focus was multiplied twice).
- `exa_game::power_save` (new): the live save walk (`saved_powers`) for
  the player's placement (the level section's character walk writes no
  running powers, as the original's: its only writer is the save game's
  placement writer), and the load
  (`restore_running`, `restore_saved_powers`, `give_player_powers`).
- A loaded persistent Sense Minds' activation starts the sight overlay with
  no end (`mind::draw::start_sense_overlay`), as its cast does.
- `exa_game::mind::runtime::tick_minds`: a loaded Loyalty or Command Dead
  finds its target by character id and puts its marker back.
- `exa_game::load_reset` (new): the GUI reset on every level change;
  `PanelIcons::clear`.
- `exa_game::save_live`: `SaveStep::SaveAndCheckpoint` from the journey's
  `JourneySave`; the player save carries the running powers.
- `exa_game::slots`: the delete's log line.

## Out of scope

- The arena's own saves (match board, hub return; the `wirearena` lane):
  the arena manager's placement gets no Loyalty target names.
- The restored Shell's aura visual (the activation's effect node): the
  shell's damping, reserve and bar entry come back, its aura does not.
- The per-locale indoor fog presets, the grid environment set-up, the
  menu scene and the arena reload of the save unit (their callers'
  branches are not ported).
- The header camera pair (F415).
