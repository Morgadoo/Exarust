# `exa_script`

Story scripting and dialogue: the role and dialogue script interpreter,
roles and relations, topic memory, the conversation engine, speech
bubbles, scripted objects, and the story/journal state.

**Phase:** Phase 7 (see [`docs/PLAN.md`](../../docs/PLAN.md)).

## Modules

| module | what it holds |
|---|---|
| `interp` | the one-pass text-walking interpreter: statements, terms, `[n]` actor slots, members, chained `Role(id)` objects, conditionals and `{ }` blocks, conditions, `{…}` line substitution |
| `builtins` | the `Actor` and `Role` classes and the globals, and the `Host` trait they act through |
| `cast` | characters as scripts see them: the shared role list (`exa_formats::roles::HeldRole`, inherit and reference counts), relation records with relation roles and memory blocks, AI contacts |
| `topics` | topic memory words: cooldowns, activation counts and windows, the memory load/store (with the original's shifted load) |
| `bubbles` | speech bubbles: durations, the queue per speaker, CR splits, fades, the hearing stimulus |
| `dialogue` | the conversation engine: the role-by-role search, conversations, entry-node starts and calm filters, the node evaluator (both player slots, the original's mood tests), branches and links, ending, event nodes, topic offers, the reply panel's list and pick, the narrator, witnessed and broadcast events, `StartDialogue` and the scripted-object script entry |
| `objects` | using a scripted world object (its script actor slot, role and sound), its glow pulse, the raise-dead revelation, the crystal drain sources and kinds |
| `story` | campaign order, the journal page |

`ScriptPlugin` loads `charroles.rdb` and `narrator.rcd` (`Scripts`) once
the installation is open, keeps the session's `Conversations` (engine,
cast, player, world clock base, locale) and steps the engine every frame on
`Time<Virtual>`. Messages: `Narrate`, `Talk`, `PickReply` in;
`ScriptEffect` (game events, inventory, checkpoint saves, witness roles,
`StartDialogue`, script errors) out.

Formats: [`docs/formats/OTHER.md`](../../docs/formats/OTHER.md#roles-charrolesrdb).

## Not yet

- Drawing the bubbles and the reply panel (placement, wrapping, the height
  animation); the AI that triggers greetings and events (contacts, arousal,
  hearing) and fills the cast; the attitude rule; zone markers (the rule is
  in `builtins::zone_match`, the world query is the app's); witness line of
  sight and the role grant (the range rule is in
  `builtins::witness_range`, the level walk is the app's); the AI's
  role-apply step; saving topic memory and relations with the character;
  the journal window; loading-screen tips (no routine of this packet
  produces them).
