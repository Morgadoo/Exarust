# Design

## Approach

`PlayerData::write_ordered` sorts the known chunk ids into the mode's
writer order and keeps unknown ids last, so a session's chunks can be set
in any order and still land as the original writes them. The order lists
are constants (`STORY_CHUNK_ORDER`, `ARENA_CHUNK_ORDER`); the arena list was
corrected from the disassembly of the writer, which calls the company and
extra writers before the employees'.

## Decisions

- **Order by the writer, not by id.** The first port ordered the arena
  tables by chunk id. The writer's call sequence is the evidence; the order
  test now spells it out.
- **A read-only inspector instead of a copy of saves.** The user's real saves
  are not copied anywhere (not into the repository and not to the build
  worker). `exa-tool saves` runs on the machine that holds them and the
  env-gated test (`EXANIMA_SAVES`) does the same check.
- **Not wired into the running game.** The game has no live session, so a
  quick save would write an empty package. The wiring waits for the level
  and company state to exist.

## Open questions

- The old-format reads of the merchant and tournament chunks take the old
  flag as a parameter; their layouts for data version below 9 are not
  modelled.
