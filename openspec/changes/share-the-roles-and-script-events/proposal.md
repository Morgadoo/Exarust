# Proposal

## Why

Phase 7 (M8 "story act 1") had two copies of the character role list (the
AI's and the script layer's) and two script runners (the AI's statement
splitter for init scripts beside the script interpreter), and the event
side of the dialogue unit (near activation, witnessed and broadcast events,
`StartDialogue`, the event node's role checks, zone markers, the narrator
text message) was partly a stand-in.

## What Changes

- One role list: `exa_formats::roles::HeldRole` with its add, remove and
  stored-word read, used by `exa_ai::roles` and `exa_script::cast`; the
  weight share is the double-precision one the original computes.
- Role init scripts run through `exa_script`'s interpreter; the AI host
  exposes the two activity slots and the role's variables, and reports
  interpreter errors.
- Topic events as the original files them: the near activation tests the
  contact's role, the witnessed event the actor's target and the contacts'
  role (entry = target, key = role), the broadcast walk keys role-to-role
  blocks by the role, and each store is followed by the role-set check
  ("Invalid role identifier.").
- Mood mask and code with the original's ordered compares (NaN fails
  each test); Trust, Concern and narrator durations rounded to nearest.
- `StartDialogue` starts a dialogue between the script's first two actors
  unless the actor sits in any active conversation; the event node keeps
  the conversation state and validates both actors after a branch runs.
- `InZone`/`InLocale` compare ids in 64 bits; zone markers are tested by
  their 3D distance and box slack; `WitnessRole` selects characters within
  18 m that the caller sees.
- Scripted objects run with the player as their one actor slot; a socket's
  drain clamps at 0 and the source stays while its charge flag is set;
  the object pulse channels, the second player slot and a plain narrator
  message are added.

## Capabilities

### Modified Capabilities

- `dialogue-scripts`: rounding, mood tests, events, `StartDialogue`,
  zones, scripted objects.
- `npc-idle`: init scripts run through the interpreter.

## Impact

- `exa_formats::roles`, `exa_ai::{roles, activity}`, `exa_script::{cast,
  builtins, dialogue, topics, bubbles, objects}`, `ScriptPlugin`.
- `docs/formats/OTHER.md` (topic memory at run time), the `exa_script`,
  `exa_ai` and `exa_formats` READMEs.
- Evidence: REA on `Exanima.exe` 0.9.5 (assembly of the mood routines, the
  near-activation, witness and broadcast routines, the memory load, the
  dialogue search, the event node, `StartDialogue`, the greeting gate,
  `Trust`, `Concern`, the bubble duration, `InZone`, `InLocale`,
  `WitnessRole`, the add-role routine, the role-set validator, the
  `GameEvent` global, the object script run and the socket drain); corpus:
  every shipped script and init script, Derrin's topics, the narrator and
  every dialogue link of the installed `charroles.rdb`.
- Out of scope: the AI filling the cast and contacts, line of sight, the
  level walks behind `WitnessRole` and `InZone`, consuming `ScriptEffect`
  in the game, drawing bubbles and the panel, the item and container walks
  of the crystal drain, the relation half of the role-set check.

## Phase

Phase 7, M8 "story act 1": the role interpreter and the dialogue engine.
