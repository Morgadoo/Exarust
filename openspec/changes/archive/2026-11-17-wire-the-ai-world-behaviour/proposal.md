## Why

Roadmap task B4 (lane `aiworld`, triage lane L05 "AI world behaviour"): the
AI's world behaviour rows the open ledger still held. Script relation roles
never reached the AI's relations, a Mask Mind hid nobody, the placements
left the AI's heading where it was, the player's footwork read the energy
where the original reads the tone, the dialogue's AI events had no caller,
and the sentry, the scene re-entry and the follower arrival each missed a
few writes.

## Evidence

REA (Exanima.exe 0.9.5), the assembly where order and constants matter:

- The script `AddRelation` and `RemRelation` act on the actor's character
  record, the one the AI reads; each checks the roles and the relation roles
  toward the partner afterwards and reports "Invalid role identifier." under
  "Add Relation" or "Remove Relation" (a remove only when it removed).
- The run-time dialogue's memory store writes the relation record back.
- The behaviour's heading store (class slot 23) of the AI: the aim snap
  from the frame's forward (already ported at the attach) and the route
  stop; the placements call it after writing the facing.
- The player's combat movement word runs the step dispatch only while the
  body may act: condition mask 0x82C1, tone at least 0.7, support above 0.7
  in double, no trailing-foot request.
- The perception reads a seen body's masked flag and looks its status list
  up for the Mask Mind id (low word 0x102), taking that entry's two factors,
  else 0.9 and 0.8.
- The target pass's round-robin refresh calls the dialogue's AI events with
  the contact; the events callbacks read and clear the contact's event word
  (Aggressor 1, Engage 2, Intimidated 4, Threatened 8); the concern setter
  raises 4 and 8 and the integration calls it.
- The sentry tick soft-stops a following body and copies the frame's forward
  into the body's look and aim directions.
- The deferred scene re-entry zeroes each contact's seen-tick count.
- The follower arrival frees the current level relation table and raises
  the behaviour's readiness to at least 5.

## What changes

- `exa_script`: a cast character queues its relation changes
  (`RelationEdit`) from the scene host and the topic memory store.
- `exa_game::script_cast`: the cast mirror replays them on the AI's
  `Relations` through the bridge's writes, with the reports.
- `exa_ai::brain::Npc::placed`, called by `exa_game::limp`'s placements.
- `exa_game::footwork`: the step gate is the can-act gate.
- `exa_ai::combatant::MindPowers::mask`, fed by `exa_game::mind::runtime`,
  read by the perception.
- `exa_ai::contacts`: one event word; `exa_game::role_scripts::run_ai_events`.
- `exa_game::golem_body::stand_sentries`, `exa_ai::presence::check_presence`,
  `exa_game::follower_travel::restore_arrivals`.

## Out of scope

The flee's combat-state run after a failed plan (the combat tick runs
before the flight and would enter the combat state), the zone check's
order, the roam's avoided room, the scripted focus, the former-level
relation migration, the saved record's topic memory and the arena's AI
words stay open.
