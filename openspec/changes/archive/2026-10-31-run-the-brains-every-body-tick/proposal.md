# Proposal

## Why

The aischedule lane found that Exarust ran the AI brain chain (the target
pass's turn, the perception, the AI tick, the duellists' combat state and
the walks) once per frame, in `Update`, on the latest body tick, while the
original runs its AI from every body tick. At about four body ticks a frame
(20 FPS) each brain's target pass came n frames apart (about 4n body ticks)
instead of every n body ticks, and every AI timing stretched with the frame
rate.

Phase 6, AI: the AI tick rate. No docs/PLAN.md edit in this change (the
main session owns it).

Evidence (REA, Ghidra on `Exanima.exe`): the physics thread's step (the
pre-tick hook list, the step counter and game clock it advances after it,
the world tick's sector steps), the sector step's character body ticks (the
last character first, the player last), and the body tick's behaviour call
(the player's behaviour or the AI's tick, through the behaviour's virtual
tick, after the daze and chest bookkeeping and before the falls, the
movement decision, the body turn, the locomotion and the feet). Recorded in
`context/artifacts/2026-10-30-tickchain/README.md`.

## What Changes

- `exa_ai`: a `BrainTick` system set in `FixedUpdate` (one exclusive
  system that runs the chain's own schedule, `BrainChain`, on a single
  thread), in
  `exa_physics::body::TickBehaviour` (new: the first set of the fixed step's
  body work), after the presence check and the daze's stance, before the
  posture, the character controllers (`exa_physics::CharacterControllers`,
  new), the body pre-step and the character ticks. The roles, the rotation,
  the perception, the AI tick, the duellists (`DuelSystems`), the flight,
  the entering turn and the idle walk run in it, once per body tick. The
  forget broadcast stays a frame system (it reads the frame's removals).
- `exa_ai::brain::ai_tick`: the fighters' places and room cells are
  gathered once a run instead of once per brain (same values: nothing in
  the tick moves a fighter's global transform).
- `exa_ai::pass_turn`: the target pass's counter is the step counter this
  tick's brains read (the body ticks run so far plus this one); the debug
  lines name the body tick.
- `exa_game`: the AI-tick parts the game owns join the brains' chain: the
  follow and the search (`npc_search`), the sentry (`golem_body`), the
  encounter scripts (`role_scripts`, after the AI tick that reported them)
  and the duellists' power system (`thaum::cast`: the answers before the
  combat state, the cast after it). The attack-hook answers run after the
  brains' tick. The power factors stay a frame system.
- `exa_physics`: the `CharacterControllers` set over the controllers'
  register, sweep and world step, and `body::TickBehaviour` before it, the
  pre-step and the grip build; the mechanisms' registration runs after it.
- Ordering so that nothing shares data with the brains' tick unordered:
  the grips and the hands' words (`exa_character`), the held weapons' sync
  (`exa_combat`) and the kinesis pick (`exa_game`) run after
  `TickBehaviour`; the entering cast reset runs before the brains' tick.
- Debug aids: `--schedule-ambiguities` (the fixed step's unordered pairs
  that share data, logged when the schedule is built) and a `brains` line
  in the frame profiler (`exa_ai::BrainTickClock`: the brains' tick's wall
  time and its split by part).

## Out of scope

- The brains still run over every brain ahead of the character ticks, not
  inside each body's own tick between the other characters' ticks (as the
  player's input already does).
- The positions the AI reads are the frame's propagated transforms, so
  within one frame's ticks they do not move (as before this change).
- The navigation footprints are kept in `FixedPreUpdate`, before the
  brains' tick, so their route gate reads the last tick's route.
- The route planners' turns keep their own counters (they now count ticks).
- The AI manager's frame (the rise, the zones, the alarm zones) stays a
  frame system, as the original's manager hook runs per frame.

## Impact

- `crates/exa_ai/src/{lib,pass_turn,brain,route}.rs`,
  `crates/exa_ai/README.md`.
- `crates/exa_game/src/{npc_search,golem_body,role_scripts,stages/mod}.rs`,
  `crates/exa_game/src/thaum/{cast,cast_test}.rs`, `crates/exa_game/README.md`.
- `crates/exa_physics/src/{lib,body,body_stages,grip_solve,body_door,body_slide,body_lever,body_plate}.rs`,
  `crates/exa_physics/README.md`.
- `crates/exa_combat/src/impact.rs`,
  `crates/exa_character/src/{body_words,grip_runtime}.rs`,
  `crates/exa_character/README.md`.
- `crates/exarust/src/{main,profiler}.rs`, `docs/DEBUGGING.md`,
  `docs/PERFORMANCE.md`.
- Specs `npc-perception` (the brains' tick) and `npc-steering` (the plan
  turn counts ticks).
