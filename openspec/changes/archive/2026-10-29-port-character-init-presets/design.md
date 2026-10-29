## Context

The character init (`docs/formats/RFC.md`, Stats derived at spawn) runs two chains of race-byte branches, then reads the race record again and overwrites most of what the branches wrote. Only the balance bonus, the arm relaxation, the footstep volume, the golem flag and the health and rule bits survive.

## Decisions

- The presets are a constant table in `exa_formats::character_stats` (code constants of the executable, not game data); the race record's values still come from `races.rdb` at run time.
- `exa_game::scene_bodies::character_muscles` overrides the two muscle factors from the stats; `race_muscles` stays for the viewer.
- The footstep's body gain field has no writer in the init; it stays 1 (a labelled guess) and the second factor becomes the footstep volume.
- The balance routine's foot sounds are plain functions in `exa_physics::body::feet`; Exarust's footsteps come from the sampler's plants, so wiring them would double the steps.

## Open questions

- The word the init sets to 4 for most creature races has no reader found in `exe.asm`; to be answered by an REA reference search on its writers' class.
- What the ghouls' half-scale part set-up changes; REA on the part routine.

## Risk

Humans keep their race record's factors; their footstep gain now follows the body scale (0.95 to 1.05 by height). Fael, ghouls, gobblers and golems balance and relax differently from before.
