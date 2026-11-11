## Why

Round 196's character view lane found two places where Exarust assumed a
human (the inventory drop's "race wears apparel" and the trade preview's
race word). This change sweeps the workspace for the same class of bug:
hard-coded race values, the wrong race word, off-by-one race ids and
lookups whose miss falls back to the human.

## What the original does

REA and `exe.asm` (Exanima.exe 0.9.5):

- A character keeps a pointer to its race record, laid out as the file is
  (its first flag word at `0x18`, the look word at `0x14`, health at
  `0xE4`), found by the race id the record's race byte plus one names.
- **The body equipment panel's core test** reads that record's first flag
  word: `0x2000` takes only Power Crystal AL, `0x1000` only crystal A, and
  neither while one of the five crystals is worn.
- **The character's model build** loads the record's worn items only when
  the race's look word has bit `0x4` (human, skeleton, ancient, ogre and
  the manipulator in the shipped table).
- **The Mind casts** read the race record's word at `0xEC`: 1 or 2 is a
  living mind, 4 or 5 an undead one, anything else no mind the casts act on
  (shipped: the human frame 1, the ogre 2, the Herath creatures, the
  manipulator and the demon 4, the wraith and the golems 0).
- **The underwear rule** gives item 8 only to race byte 0.

## What Changes

- `exa_game::panel_live`: the core test looks the race up by byte + 1
  (`core_race_flags`); it read the race whose id equals the byte, so golem
  guards took no crystal and the golem boss took the guards' one.
- `exa_formats::races`: `Race::wears_apparel` (look bit `0x4`) and
  `RaceStats::mind` (`0xEC`); `CharacterStats::mind` carries it.
- `exa_game::scene_bodies` and `exarust::viewer`: a body is dressed by
  `wears_apparel`, not by the human frame (bytes 0 to 2), so the
  manipulators of exanima05_sw wear their armour.
- `exa_game::mind::runtime::attach_minds`: the record kind is the race's
  mind word, not 4 for skeletons and ancients and 1 for every other race;
  the cast logs the target's word.
- `exa_character::keep_underwear`: the rule reads the body's race
  (`RaceWeaponry::index`).
- `exa-tool races` prints the look and mind words.

## Capabilities

### Modified Capabilities

- `character-stats`: the race record's dressing bit and mind word, the
  core test's race lookup and the underwear rule's race.

## Impact

- Code: `exa_formats::{races, character_stats}`, `exa_character`,
  `exa_game::{panel_live, scene_bodies, mind}`, `exarust::viewer`,
  `exa_tool`.
- Simulation: Mind casts on non-human targets answer differently; ogres
  and manipulators with worn items are dressed. Levels without those are
  unchanged (lockstep traces identical).
