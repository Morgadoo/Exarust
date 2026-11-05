## Why

Phase 5 (arena), XA_Arena, the playerctl lane (round 77): the arena's
recorded gaps that the shipped game reaches — the candidates' outfits and
their gear fee, the specialist placed at its hire, the tournament day's
clearing of unready entries.

## Evidence

REA: the hire-list candidate routine (role draw, body generator, the
specialist profile bytes and cleared hands, the combatant's level outfit,
loadout copies and trims), the hire routine's placement call for a
merchant or physician, the day end's tournament-day branch (drop unfit,
then the team-size and weapon-row checks per event), the kit value.

## What Changes

- `hub_windows`: `HireFacts`, `dress_candidate`, `specialist_profile`; the
  price and the hire carry the outfit; `Member::outfit`.
- `arena_hub_live`: `hire_facts_of` and its read at the hub;
  `place_hired_specialists`.
- `hub_forms`: `clear_unready_entries` at the tournament day's end; a hired
  employee's record wears the outfit; the unfit pass reads the manager as
  participant 0 (also at the match return).

## Out of scope

- Saving the hire list's candidate records and the week's event rows (the
  character-export codec).
- The candidate records' item instances (quality, condition).
