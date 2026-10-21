# Proposal

## Why

The cast rules, the power bar and the active power classes were plain
tested functions with nothing calling them: the player could not cast. The
bar's cast entry, the front ends, the creators and the body's active-power
walk are now called from the running game.

## What Changes

- `thaum::cast::CastPlugin`: the bar commands (`C`, a digit, `Q`, the
  arrows) from the hotkey handler reach the bar's cast entry with the
  slot's word as it stands; the entry dispatches by the word's domain byte
  to the Force, Energy and Displacement front ends (Mind is refused).
- The creators of the started powers: the shell, kinesis, throw, strikes,
  bubble and barricade (Force); shock and shock weapons (Energy); vanish,
  the long door, constant displacement and the summon (Displacement), in
  the original's order (cast motion and power id, then the power object,
  then the reserve). Pressing vanish again tells the running vanish to let
  go.
- The body's tick walks the active list with control event 0 (event 2 for a
  power marked ending), removes a finished power with the last entry in its
  place and frees what it held.
- Live effects for shock arcs, the barricade and the constant displacement
  wisps; cast motions played by action name.
- The tree screen and the caster share one record: the caster takes it back
  when the tree closes; the screen's Escape closes the bar.
- `--powers id,id,...` grants the player powers for a run.

## Capabilities

### New Capabilities

### Modified Capabilities

- `thaumaturgy`: the casts and active powers run in the game.

## Impact

- `exa_game::thaum::cast` (new), `exa_game::hotkeys`, `exa_game::power_screen`
  (the stand-in body is gone), `exa_anim::Motions::action_name`,
  `exarust` (`--powers`), `docs/THAUMATURGY.md`, `docs/PLAYER.md`,
  `docs/DEBUGGING.md`, `crates/exa_game/README.md`.
- Evidence: the original's bar cast entry (dispatch by `word & 0xff00`, the
  "not now" queue, the close on success), the Force, Displacement and shell
  creators (cast motion first, then the power object, then the reserve) and
  the body tick's active-power walk (control event 0; event 2 only for
  death and collapse), read with REA from `Exanima.exe` and the packs.
- Out of scope: the effect entities for blast, bolt, wave, torrent, barrier,
  dome, shell, shock burst, vanish and the demon; kinesis and throw targets
  (no cursor body pick); Mind casts (they need a target pick); the player's
  saved record and the grant routine; sounds and HUD; retrying a queued
  slot; combat reading the shell damping and strike factor; the AI's
  chooser.

## Phase

Phase 7, game modes and content: thaumaturgy, the powers' callers.
