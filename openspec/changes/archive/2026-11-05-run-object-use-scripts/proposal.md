# Proposal

## Why

Round 53 left two gaps around scripts that the player's actions start:

- A scripted device's use (usable class 2, kind 3) pulsed its glow and
  played its sound, but its use script never ran. The one shipped use
  script, the Pedestal Hand's `WitnessRole(Operative, $E1D1)` in
  `exanima67`, therefore never granted its role, and `WitnessRole` measured
  its 18 m reach from the player as a stand-in for the operative object.
- Wiring backlog item U3: `StartDialogue` was modelled in `exa_ai::roles`
  as a returned `DialogueRequest` that nothing consumed, while
  `exa_script`'s builtin started the conversation between the calling
  script's first two actors; and the Ctrl/double-click interaction on a
  living character never ran the roles' interaction condition (the hand
  passed a fixed "fails").

Phase: Phase 6 (story mode: NPC dialogue and scripted events) of
docs/PLAN.md; no PLAN.md edit in this change (the owner's).

Evidence (REA, Ghidra on `Exanima.exe` 0.9.5, checked against the
assembly): the object use and its script run (actor slot, owner, text
accessor), the operative section's script accessor, `WitnessRole` and its
wrapper, the roles unit's registration, its `StartDialogue` and the
encounter that writes its static context, the dialogue's AI entry the
builtin calls and the search callback, the controls' interaction hook, its
condition test, and the character inventory window it opens. Corpus: grep
of `objdb.rdb` and `charroles.rdb` in the installed build.

## What Changes

- `exa_formats::objdb`: an operative section keeps its use script (length
  at body `0x38`, text from `0x3C`, none under 1).
- `exa_world::ObjectSections` carries it to the placed object.
- `exa_game::object_click`: a scripted device's use runs its script first
  (the player as the one actor, no running role, the device as the
  operative), then pulses and sounds. A `WitnessRole` from it becomes a
  `WitnessGrant` at the device's place; the script's other effects go out
  as script effects.
- `exa_game::script_cast`: `WitnessRole` grants from the operative's place;
  a `WitnessRole` effect from a script without an operative grants nothing.
- `exa_script`: the engine keeps the roles unit's encounter context
  (owner, partner, role, partner role); `StartDialogue` in any script starts
  the named node between that pair; `use_script` takes no role;
  `run_encounter_script` writes the context.
- `exa_ai::roles`: the unused `DialogueStart`/`DialogueRequest` model is
  removed; the docs point at the engine.
- `exa_game::character_use` (new): the interaction on a living character
  runs its interaction condition toward the player (none fails, empty
  passes, else result above 0), and a failed one still opens for a Loyalty
  or Command Dead marker the player cast; refused while collapsing or
  possessing. The window it opens (the character's inventory) has no
  Exarust view and is logged.
- `exarust --encounter-script TEXT`: a debug tool running TEXT as one more
  encounter script.

## Out of scope

- The character inventory window bound to another character (no Exarust
  view; the script `ShowInventory` of another character is left the same
  way).
- Whether the roles unit's encounter run sets the running role of its
  script context (no write to it was found; Exarust keeps passing the
  role).
- Starting the dialogue inside the `StartDialogue` call rather than when
  the script returns (unchanged ordering).
- Whether the AI manager's character list that `WitnessRole` walks holds
  the player (Exarust walks every character with roles, the player
  included).
