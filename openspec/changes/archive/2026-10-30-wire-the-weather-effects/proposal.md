# Proposal

## Why

Wiring backlog item E1: the scene environment's weather tick returned its
events (stop the rain effect, start or stop the rain loop, a thunderclap)
and `exa_world::environment` dropped them, so no rain was drawn and neither
the rain loop nor thunder played, although the weather model, the rain
effect's drops and the lightning flash were ported. The outdoor arenas'
weather ambience (the wind bed) had its rule but no caller.

Phase: the world and audio milestones of docs/PLAN.md (environment
effects); no PLAN.md edit in this change.

Evidence (REA, Ghidra on `Exanima.exe`): the scene environment's rain
setter (it loads `rainfall`, takes a voice whose flag word becomes
listener-relative, creates the rain effect and, above 0.75, loads the
`Thunder` set), its weather tick (it hands the moving rain amount to the
effect's intensity setter, copies the effect's two sound figures into the
voice, starts and stops it, and plays a clap through the set player at the
strike's position, listener-relative), the strike routine, the rain
effect's intensity setter, update and frame routine (its material record,
blend word, vertex formats, the 2° box turn and the field-of-view scale),
the audio environment's voice allocator, voice start and set player, and
the arena setup's ambience branch with its per-arena tables.

## What Changes

- `exa_world::scene_env`: the tick gives the rain effect its intensity on
  each tick the rain moves; a clap carries the strike's position.
- `exa_world::environment`: the tick's events are kept on the runtime
  (`SceneEnvRuntime::events`, `now`) and the tick runs in a public set
  (`LiveEnvironment`).
- `exa_render::rain` (new): the rain program as a Bevy material over the
  frame copy of the transmissive pass.
- `exa_game::weather` (new, live): creates, steps, draws and frees the rain
  effect, writes its sound figures back, drives the rain loop's voice, plays
  thunder, and sets an arena's weather ambience once its scene is in.
- `exa_audio`: `LevelAmbienceBed`, the bed override the arena setup writes.
- `exa_game::effects::rain`: the "bright setting" is the pause counter; the
  box's turn and scale.

## Out of scope

The original's per-supersample-corner rain passes and the reflected pass;
the shared engine generator for the thunder variant; the editor's own
environment ambience; the arena progression (S4) and a driven time of day.

## Impact

`exa_world`, `exa_render`, `exa_game`, `exa_audio`, `exarust` (the bed
override reset on a level load); docs/RENDERING.md, crate READMEs.
