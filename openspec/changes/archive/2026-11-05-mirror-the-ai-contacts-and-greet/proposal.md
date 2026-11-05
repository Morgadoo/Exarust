# Proposal

## Why

Wiring backlog item U2. The conversation engine's mood bands, `Trust`,
`Concern`, `AddConcern`, the speech hearing, `ActivateTopicN`, the witness
broadcasts and the memory fragments' witness event all read a character's
AI contact list, but the script cast never received the brains' contacts:
every one of those read an empty list. The target pass reported greetings
that nothing ran, the script `Report` had no reader, and the cast's
arousal was the personality's constant starting value instead of the
brain's live arousal word, so no AI could ever greet.

The backlog's premise was partly stale: `WitnessRole` (18 m grant) and
`ShowInventory` already have readers (rounds 46 and 48), and
`ActivateTopic` runs inside the engine. `StartDialogue` stays with U3.

Phase: Phase 6 (story mode: NPC dialogue and scripted events) of
docs/PLAN.md; no PLAN.md edit in this change.

Evidence (REA, Ghidra on `Exanima.exe`, checked against the assembly): the
dialogue's mood routine (contact level, concern, hostility, the trust
query), the trust query itself, the bubble's hearing routine (the
contact's noticed level and attention offset), the target pass and its
greeting call (gates and order after the encounter), the dialogue's AI
greeting entry (arousal word), the personality routine (arousal word set
to 10) and the contact integration that drifts it, the broadcast, the
witness event and its one caller in the story event handler, `WitnessRole`
and its wrapper (the `Operative` check and the operative's place), the
dialogue's role-set report and the engine message hook's level.

## What Changes

- `exa_script::cast::Contact`: the contact value is the AI contact's
  hostility, the state word its classification level, the hearing its
  noticed level and attention offset (renamed and documented).
- `exa_game::script_cast`: each brain's character contacts are mirrored
  into the cast before and after the fixed step; a concern or a hearing a
  script or a line changed goes back to the brain first. The player's
  contacts are its behaviour's list. The cast's arousal is the brain's
  readiness word. A script `Report` is logged as a level-4 message.
- `exa_game::role_scripts`: the greeting a target pass reports runs after
  its encounter scripts, through the dialogue engine's greeting, with the
  brain's contacts synced first.
- `exa_game::game_event_live`: a memory fragment's witness event reaches
  the player's contacts holding its role.
- `exa_ai`: a brain's readiness starts at its personality's arousal (10).
- `exa_script::dialogue::witness_event` returns how many witnesses it
  told (for the log).

## Out of scope

- Placed objects' use scripts (the only shipped `WitnessRole` caller, the
  Pedestal Hand) do not run; the grant keeps the player's place as a
  stand-in for the operative's.
- The round-robin AI events the target pass offers (the refresh slot) have
  no reader yet.
- `StartDialogue` (U3); trust toward a character that is not a contact
  (mood and `Trust` read 0 there; the original asks the relation).
- The player behaviour's contact words other than the other character.
