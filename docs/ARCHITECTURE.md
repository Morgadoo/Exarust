# Architecture

## Layers

```
┌──────────────────────────────────────────────────────────────────────┐
│ exarust (binary)   exa_tool (CLI)                                    │
├──────────────────────────────────────────────────────────────────────┤
│ exa_game   modes (Story / Arena / Practice), app states, saves       │
├───────────────┬───────────────┬───────────────┬──────────────────────┤
│ exa_ai        │ exa_combat    │ exa_script    │ exa_ui               │
├───────────────┴───────┬───────┴───────────────┴──────────────────────┤
│ exa_character  exa_items                                             │
├───────────────────────┴──────────────────────────────────────────────┤
│ exa_physics (fixed step)   exa_anim (targets)                        │
├──────────────────────────────────────────────────────────────────────┤
│ exa_world   exa_render   exa_audio                                   │
├──────────────────────────────────────────────────────────────────────┤
│ exa_assets  — Bevy bridge: GameData resource, asset source, loaders  │
├──────────────────────────────────────────────────────────────────────┤
│ exa_formats — pure parsers         exa_rpk — packs, project, VFS     │
└──────────────────────────────────────────────────────────────────────┘
```

Rules:

- `exa_rpk` and `exa_formats` do not depend on Bevy. They take bytes and
  return plain data, with errors instead of panics on malformed input.
- Only `exa_assets` turns parsed data into Bevy assets (`Image`, `Mesh`,
  materials, audio sources). Gameplay crates consume Bevy assets and
  components, never raw formats.
- Each gameplay crate exposes one `Plugin`. `exa_game::GamePlugin` composes
  them. The binary adds `DefaultPlugins`, `GamePlugin` and the CLI.

## Schedules

| schedule | content |
|---|---|
| `Startup` / `OnEnter(state)` | open install, load level, spawn entities |
| `FixedUpdate` (83 Hz, the original's 12 ms tick, deterministic) | input → control intents → AI intents → character controllers → pre-step (`BodyPreStep`) → character ticks (`CharacterTicks`: each living character, the player last, runs the nine stages of the `CharacterTick` schedule before the next one starts: upkeep, readiness, anchor, behaviour, posture, attack, movement, sampler, carry) → body solver (blows, integration, iterations, contacts, friction, post-step, with the registered `SolverStages` at the original's points; `BodySystems`) → per-character post-step (`BodyPostStep`: combat resolution, injuries) |
| `Update` | camera, UI, audio triggers, streaming |
| `PostUpdate` | body parts drawn between their last two ticks, transform propagation, skinning, visibility |
| Render | Bevy render graph + custom nodes (shadow cubes, GI, post) |

Player input and AI both produce the same `ControlIntent` component. The
simulation never reads raw input. This keeps AI on equal footing with the
player and makes replays a recording of intents.

## Data flow for an asset

```
Steam install ─mmap─► MappedArchive (exa_rpk)
     └─ bytes ─► exa_formats::{rfc,rfi,rsq,…} typed data
          └─ exa_assets loader ─► Bevy Asset (Mesh / Image / Material / AnimationClip)
               └─ exa_world / exa_character spawn entities referencing handles
```

Asset paths: `exanima://<Pack>/<entry>` (e.g. `exanima://Objlib/afr_cornflag_a01`),
with nested packs as `exanima://Resource/common.rml/bontile`.

## State and determinism

- All simulation state is ECS components and resources. There are no globals
  and no wall-clock reads inside `FixedUpdate`.
- Random numbers come from a seeded, per-match RNG resource.
- A slow frame catches up at most 4 fixed steps (48 ms of game time); the
  rest of the backlog is dropped and game time slows, as the original drops
  its time-critical thread's backlog. Characters and their body parts are
  drawn between their last two fixed steps, so they move smoothly at any
  frame rate, and the same ticks give the same bodies at any frame rate.
  Events sent from frames (blows) are queued for the next tick.
- Props and items with a rigid body are bodies in the same solver, asleep
  where the level placed them until something wakes them; only awake
  bodies are redrawn.
- Today input steering, the character controllers and the bodies run in the
  fixed step. AI, balance, combat and the clips of bodies that are not
  physical still run once a frame.
- Physics uses a fixed step with a fixed iteration count. Floating-point
  determinism is expected per build and per platform, not across platforms.

## Units and axes (to be confirmed in Phase 1)

Original geometry appears to be in centimetres. Bevy uses metres, Y-up and
right-handed. The conversion happens once, in `exa_assets`, never in gameplay
code.
