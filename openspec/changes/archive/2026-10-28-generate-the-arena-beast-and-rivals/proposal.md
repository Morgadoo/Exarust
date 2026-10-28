# Proposal

## Why

docs/PLAN.md, Phase 7 (game modes and content), arena paragraphs
(wire-the-arena-end): the Beast stood in as a human actor template, every
generated body continued the arena roll instead of the original's clock
reseed, a tournament side wore the company indexed by its own team id, the
launch pre-drew the event weapon before the record existed, and the hub
launch gave a hire profile the team id as its level byte.

## What Changes

- `exa_game::arena_records::beast_record`: the Beast's record is the Ogre race
  (profession word 0x300), combat skill 100, its tier's main-hand item and
  worn pieces, voice pitch `0.9 + 0.17 · smooth` from the first draw, voice 0,
  50000 experience on the profession grid and personality −100, 100, 0;
  `scene_bodies::BodyLook::Beast` spawns it in its race's body.
- `scene_bodies::clock_seed`: before each generated or Beast record the shared
  sequence restarts from the performance counter's low word byte-reversed.
- `scene_bodies::hire_row_plan` / `GeneratedFighter::hire_from_row`: a hire
  side armed from the event row draws its variant after the random NPC pass
  and before the outfit pass; the hub launch no longer pre-draws weapons.
- The random NPC pass's variant is the record's voice number, its 0.9…1.07
  curve the voice pitch and its appearance draw the undead voice set.
- `ArenaCareer::rivals` / `rival_field`: the week's sixteen-company field,
  drawn when its first company is 0, carried by the launch and the scene and
  saved in the tournament chunk; `arena_spawn::rival_company` dresses team
  `t ≥ 2` in the field's `t − 2`th company in the first round, the launch and
  the bracket's later fights. The launch's hire profile takes the event's tier.

## Evidence

REA/Ghidra on Exanima.exe 0.9.5: the beast spawn routine (record words, the
double-precision pitch, the tier switch, the worn-list append, the level
grant of 50000), the generic and hire spawn routines (clock reseed first,
the hire's variant pick between the NPC and outfit passes), the clock helper
(counter low word byte-reversed), the fresh tournament raise (team ≤ 1 generic,
else the field's word at team − 2 with the event tier), the field draw and the
tournament chunk's load. The runtime record's field order (voice, pitch,
profession; combat skill, undead voice, personality, search seed) matches the
character record format in docs/formats/RFC.md. The race table (`exa-tool
races`) names race byte 3 the Ogre.

## Out of scope

Company entrants' and hub specialists' saved records, the item instance's
generated quality and condition (the variant's grade 0 and the outfit's
grades), the score board and round banner, the tournament week's own clock
reseed and board generation in the live hub.
