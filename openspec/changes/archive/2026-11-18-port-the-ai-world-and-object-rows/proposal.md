## Why

Roadmap task B4, lane `aiobj`: the open rows of the triage's port lanes Q10
(AI world behaviour) and Q14 (world objects, script and audio), the ones a
player sees or hears first. A force effect's assessment reached a brain's
contact only as its alarm, from the body's first part's axis; a character
that left its level with the player lost the relations its old level's
characters kept about it; a story track restarted at every door even when
the next level plays the same one; the crystal drain ran in every mode; and
a searching follower's target pass started its notice score at nothing.

## Evidence

REA (Exanima.exe 0.9.5), checked against the assembly: the target pass
copies a contact into a scratch record, calls the entity's assessment slot
on it and copies the whole record back (the blast, wave and barricade
assessments write the noticed level, the sighting, the concern target, the
soothe term, the alarm, the seen word and the stamp; the face terms read
the body's look direction and its frame forward); the follower flush
writes the record's origin (the level left and the local id), and the last
attitude moves the old level's relation into the global table under the
whole id and answers it from the moved record, while the later lookups key
it by the masked id; the story level hook compares the picked track with
the playing one on 15 characters and skips the play and the ambience
switch when they match; the story frame hook calls the drain once a game
second after its dying and playing gates; the target pass starts its
notice score at the idle activity's word while the follow state's busy
byte is set.

## What changes

- `exa_game::effects::danger::Seen::of_contact` / `write_contact` and
  `Hazard::assess_into`; `thaum::live::see_force_effects` assesses into the
  brain's contact from the body's look and frame forward.
- `exa_ai::roles::FormerLevel` on characters placed with an origin; the
  perception and the force effects' caster relation pass it; the moved
  relation answers its first ask.
- `exa_audio::StoryMusicHook` and `story_hook_plays`; a story door keeps the
  level music for the hook to compare.
- `exa_game::flow::Flow::story_hook_runs` gates `crystal_drain::drain_sources`.
- `exa_ai::brain::Npc::activity_busy` and `search::NOTICE_FLOOR` seed the
  target pass's notice score.
