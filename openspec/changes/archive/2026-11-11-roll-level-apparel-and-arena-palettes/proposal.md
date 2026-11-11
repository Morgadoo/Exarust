## Why

Item description phrases (rated 81%): the palette lane (round 174). The
rating left open the part colour by dye class (modes 1–0x40), the NPC
outfit palette (`roll_around`), the value and desirability helpers'
callers, the engine's shared generator for the card roll and the AI
search's duplicate desirability and protection copies.

## Evidence

REA (Ghidra on `Exanima.exe` 0.9.5, checked against the assembly):

- The part colour's two callers are the factory's area rolls (the first
  areas of a fresh item's stored choices and the random roll); both load
  the mode argument with `xor edx, edx`, so modes 1, 2, 4 and `0x40` are
  unreachable; mode 0 is the factory's colour roll, already live.
- The plain palette roll has six callers: the arena's two outfit passes,
  the save unit's archive pass (the world-load walk), the grade copy, the
  random-table pick and an editor. The archive pass is the hook the save
  unit installs for a level read from the archive: the tier migration,
  then the walk (an item body with an armour type rolls the palette at
  kind 1 for its palette byte's wealth and signed variance and runs the
  apparel walker: seed, protection bytes, the layer and variant roll, the
  condition; a weapon runs the weapon walker), then the container fill.
- Each palette, layer and weapon roll reseeds the shared generator from
  the clock entropy word.
- The profile outfit pass calls the palette around the profile's bytes 5
  and 6 (kind 2 for a rank-five fighter with an empty off hand, else 1)
  when the profile's flag 8 is set (a forced company outfit), else the
  plain roll (kind 2 or 0); the level pass the plain roll. The palette's
  match flag (set only around a company's colours) picks mode two over
  three for the max-rank armour.
- The protection value's bone factor is `w·(√(c·0.01)·0.005)` in double;
  the AI search's copy multiplied `(w·√…)·0.005`.
- The shield value's one caller is the character view's weapon drop on a
  mind-controlled character (the bind and command will test); the
  handling sound's one caller is the AI's wear executor.

## What Changes

- `exa_game::archive_pass`: the archive pass live for a level read from
  the archive, before its characters are dressed; the live database takes
  the migrated tier (`LiveObjects::load_level_migrated`).
- The arena's outfit passes roll the palette (`GeneratedOutfit::palette`)
  and the company palette's match flag picks the max-rank mode.
- `exa_formats::item_value`: desirability and protection shared by
  `exa_items::instance` and `exa_ai::search`.
- The part colour's unreachable modes and the card roll's generator are
  documented, not wired.

## Impact

exanima01: 253 of 490 level records change (129 apparel); every placed
character's worn pieces take rolled wear and variants; four NPCs' turn
rates follow their new encumbrance from tick 1. The duel is unchanged.
