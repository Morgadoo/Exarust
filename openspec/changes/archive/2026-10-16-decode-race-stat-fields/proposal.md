## Why

The original's character init, combat, AI and sounds read more of each
character's `races.rdb` record than Exarust parses: the clip clock's base
rate (1.5 for the ghouls), the body's size, scale and mass divisor (an
ogre is 1.5× a human, the harvester 1.8×), the empty hands' reach, the
race's scaling of held weapons, its AI's attack bonus and perception, its
footstep sounds and voice, and two more flag words (red regeneration,
claws, a quadruped's strikes, a hovering wraith). Every later change that
derives a character's stats at spawn (M41), times its clips (M14), holds
weapons (M21, L12), steps (M16) or regenerates (M43) reads them, and the
master plan makes this change the sole owner of `races.rs` so they land
once.

Phase 3 (Bodies), milestone M3 "standing man"; it also serves Phase 4's M4
"first duel". This is change M40 of the master plan
(`context/re/master-plan.md`, lane F, F3; body note B3).

## What Changes

- `exa_formats::races` decodes, for every race:
  - the body scalars: the clip-rate base (`+0x5C`), size (`+0x64`), scale
    (`+0x68`, with the init's cap of 1.5 as a constant) and mass divisor
    (`+0x70`), beside the body controller's factors (`RaceBody`);
  - the gait rows' brake distance, the column the original's loader fills
    with built-in values (0.5, 2, 2.5 and 0.5 m), and the sprint gait of a
    feral character: the sprint row moved toward the jog row by the feral
    level;
  - the second flag word (`+0x1C`) and the kind word (`+0x58`), with the
    bits the original tests named (rising again in the story, red
    regeneration, hovering, quadruped, clawed fists);
  - the stats (`RaceStats`): the unarmed reach (`+0x84`), the weapons'
    swing-turn and recovery factors (`+0x88`, `+0x90`), the AI attack bonus
    (`+0x8C`), the perception (`+0x110`), the held-item offset (`+0x12C`),
    the footstep sound set (`+0x144`), the voice volume (`+0x1B4`) and the
    attack voices (`+0x1C4`);
  - the original's built-in fallback for each of them when a race is
    missing.
- Lengths are returned in metres (the reach, the held offset, the brake
  distances); factors and gait rates as stored.
- `exa-tool races` prints the new fields.
- No game behaviour changes: nothing consumes the values yet. One line in
  `exa_combat`'s tests (lane C's file) fills the new field of a synthetic
  race.

Evidence:
- The installed `races.rdb`: all 18 records, every new field pinned by a
  corpus test.
- Reverse engineering of `Exanima.exe` 0.9.5 (REA, `exe.asm`, constants
  read from the image): the character builder (scale capped at 1.5, size,
  mass divisor, the height factor 1 − 0.05·h), the character init (clip
  base (83/83) × the field, the flag words to the character's rule bits,
  perception, the AI bonus, voice volume = scale × the field, attack
  voices − 1, the feral sprint blend), the race loader (the built-in
  column 0.05/0.2/0.25/0.05 and the built-in fallback record), the weapon
  pass (recovery and swing turn scaled, the unarmed reach), the moveset
  chooser and fist record choice (claws), the footstep set, the hand
  placement (offset bytes × 0.1 cm), the AI's sighting ranges and attack
  scoring (`context/re/master-body.md` §7.2–7.4, `master-animation.md`
  §3.1 and §8.1, `master-movement.md` §5.1 and §6.2, `combat-damage.md`
  §1, refreshed in this change).

Out of scope:
- deriving any character's stats from these (M41: scale and size with the
  height, reach, pins, rule bits, feral effects, voice);
- consuming them in the clocks, bodies, combat, AI or sounds (M14, C14,
  C18, M16, M21, L12, M43 and later);
- the rest of the record: `+0x6C` (1.0 everywhere, no reader found),
  `+0xE8`–`+0x108` (read by view and camera routines not traced here),
  `+0x118`, `+0x194`–`+0x1C0` but `+0x1B4`, `+0x1C8`–`+0x218`, the other
  sound names (`+0x134`, `+0x154`, `+0x164`) and the voice set names the
  loader reads (`+0x174`, `+0x184`).

## Capabilities

### New Capabilities
- `character-stats`: the race stats and flag words the character's init,
  combat, the AI and the sounds read; later lane F changes (condition,
  derived stats, regeneration) extend it.

### Modified Capabilities
- `physical-body`: the race body parameters gain the clip-rate base, size,
  scale, mass divisor and the gait rows' brake distance; a new requirement
  for the feral sprint gait.
- `asset-inspection-cli`: `exa-tool races` prints the flag words, the new
  body parameters and the stats.

## Impact

- `exa_formats` (`races.rs`: `RaceBody` fields, `Gait::brake_distance`,
  `BRAKE_DISTANCES`, `SCALE_CAP`, `RaceBody::sprint_gait`, `RaceStats`,
  `Race::stats`, `stats_of_character`, flag constants,
  `Race::regenerates_red`, `Race::clawed`)
- `exa_tool` (`races` command)
- `exa_combat` (one test line)
- `docs/formats/OTHER.md` (Races), `docs/PLAN.md` (Phase 3),
  `crates/exa_formats/README.md`, `crates/exa_tool/README.md`
