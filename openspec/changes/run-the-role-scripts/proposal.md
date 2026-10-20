# Proposal

## Why

Phase 7 (M8 "story act 1") has no role interpreter, no dialogue graphs and
no conversation engine: `exa_script` holds only the campaign order, one
dialogue choice and the journal page. The role database's scripts, the
dialogue graphs in role entries and `narrator.rcd`, the topic memory and the
speech bubbles are all unread, so no character can greet, answer, follow,
or have the narrator speak.

## What Changes

- `charroles.rdb` role records and their entries are read in full: header,
  init script, variables, attitude/tolerance, and the sections an entry
  carries (dialogue graph, encounter script, interaction condition, the
  unknown blocks, the game-event line). Dialogue graphs (role entries and
  `narrator.rcd`) are read: actors, nodes, links and nested branches.
- The role and dialogue script language runs as the original walks it:
  statements, terms, `[n]` actor slots, member calls, chained `Role(id)`
  objects, `? cond : … / …` with brace blocks, conditions and expressions,
  arguments as integers, `$hex` ids, names and actor references, `{…}` line
  substitution. The `Actor` and `Role` classes and the globals act through a
  host.
- Role lists and relation roles change as the original's add/remove
  routines do; topic memory words follow the cooldown, activation and
  window rules with the two shipped tables.
- The conversation engine: the role-by-role dialogue search, conversation
  reuse, entry-node starts with the calm filters, the node evaluator (moods,
  reply choices, random line per speaker), branches, links, ending and topic
  recording, event nodes, topic offers, the reply panel's list and pick,
  and the narrator; speech bubbles with their timing and fades.
- A Bevy plugin loads the role database and the narrator graph and runs the
  engine each frame.

## Capabilities

### New Capabilities

- `role-database`: role records, entries and dialogue graphs.
- `dialogue-scripts`: the script interpreter, its classes and globals,
  topic memory, the conversation engine and speech bubbles.

### Modified Capabilities

## Impact

- `exa_formats::roles`; `exa_script::{interp, builtins, cast, topics,
  bubbles, dialogue}` and `ScriptPlugin`.
- `docs/formats/OTHER.md` (roles and dialogue graphs), the `exa_script`
  README.
- Evidence: REA on `Exanima.exe` 0.9.5 (the script unit's statement loop,
  term, member, argument, expression, condition and registry routines; the
  `Actor` class registration and its methods; the `Role` class; the add and
  remove routines of roles and relations; the dialogue unit's search, open,
  start, node evaluator, branch, step, end, event, topic, panel, bubble and
  narrator routines; the run-time dialogue's memory load/store and topic
  gate; the cooldown, activation-delay, calm and event tables). Corpus: every
  record of the installed `charroles.rdb` and `narrator.rcd`.
- Out of scope: drawing the bubbles and the reply panel (placement, wrap,
  height animation), the AI side that triggers greetings and events
  (contacts, arousal, hearing), the attitude rule, zone markers, witness
  line of sight, the role-apply step of the AI, saving topic memory with
  the character, and the journal window.

## Phase

Phase 7, M8 "story act 1": the role interpreter and the dialogue engine.
