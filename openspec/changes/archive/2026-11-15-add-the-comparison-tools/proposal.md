## Why

Roadmap task C1 (comparison tools), with the `--new-game` flag owed from
round 205. An outdoor level drew its sun and weather from the clock, so two
runs gave two pictures and no picture could be compared with another or
with a retail capture. A new story could only start through the creator's
form (`--open-creator begin:N`) or as a play level dressed as a story
(`--story-session`, which makes no slot), so the session begin's new slot
and its first save had no unattended live check. There was no tool to tell
how far two pictures of one pose differ, or what the noise between two runs
of one build is.

## Evidence

No original function is ported or changed. The session begin and its new
slot were traced in round 205 (`make-the-new-games-slot-and-write-the-arenas-saves`);
the creator's BEGIN calls the story start and the session begin, which
`--new-game` calls in the same order. The arena career's start is the
creator's commit (game mode 8), which the flow has no starter for; the
creator files are left untouched, so `--new-game arena` is refused.

## What Changes

- `exa_world::environment::fix_seed` / `fixed_seed`: `--env-seed <n>` pins
  every outdoor preset seed of the run (the load's preset and engine seeds,
  and the preset's own fresh-seed fallback); `EXA_SCENE_SEED` still works.
- `exa_game::new_game`: `--new-game story[:N]` starts a story through
  `Flow::start_story` and `Flow::begin_session` once the first load ends;
  the play level's player stands in for the created character.
- `exa-tool picdiff`: changed pixels above a threshold, their regions,
  a noise floor from a second pair, a side-by-side and a diff picture, a
  JSON report; exit 1 above the floor.

## Impact

Debug aids only: nothing changes without the flags. Crates `exa_world`,
`exa_game`, `exarust`, `exa_tool`.
