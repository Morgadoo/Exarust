## Why

The original's body controller reads several factors and a gait table from
each character's `races.rdb` record. They set how hard the support pins pull,
the balance bonus, the stance factor that halves a standing human's muscle
tone, how far the arms relax, how much limbs are damped, and how hard each
gait pushes. Exarust parses only the race's names, models, movesets, health
and natural armour, so none of this is available. The next body changes
(link muscles, balance and support, the pose drive, the stance pins and
locomotion through the carry) all read these values.

Phase 3 (Bodies), milestone M3 "standing man". This is change C2 of the
body-controller plan. Its blocker, which record the body reads (Q2), is
answered: the loaded `races.rdb` record, with the file's own layout.

## What Changes

- `exa_formats::races` decodes each race's body parameters:
  - the support-pin factor (`+0x60`);
  - the balance bonus (`+0x74`);
  - the stance factor (`+0x78`);
  - the arm relaxation factor (`+0x7C`);
  - the limb damping ratio (`+0x80`);
  - the gait table at `+0x94`: 4 rows (walk, jog, sprint, backpedal) of 5
    floats, of which the original reads thrust, push scale, clip rate and
    the boost speed (column 2 is replaced by a built-in value per row).
- The parameters the original falls back on for an unknown race (every
  factor 1.0, an all-zero gait table) are available as a constant, with a
  lookup by character race byte that returns them when the race is missing.
- `exa-tool races` prints every race with these values.
- No game behaviour changes: nothing consumes the values yet.

Evidence:
- The installed `races.rdb` (all 18 records, read by the new parser and a
  corpus test).
- Reverse engineering of `Exanima.exe` 0.9.5: the race lookup, the loader
  that keeps the file's record and derives the per-tick gait table from it,
  the built-in fallback record, and each consumer of the five factors
  (`context/re/body-drive.md` §2, §5, §10, §12; `body-links.md` §4.3;
  `body-feet.md` §3; `body-tick-order.md` §4.2; `player-control.md` §1.4).

Out of scope:
- using the values in the simulation (C5, C6, C7, C14 and C18 do that);
- the other floats between `+0x58` and `+0x94` and the per-part values,
  sound names and the rest of the record (`+0x5C`, `+0x64`, `+0x84` and
  others are not decoded here);
- the original's per-character overrides of the balance bonus for creature
  types.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `physical-body`: a new requirement for the race body parameters the
  body controller reads.
- `asset-inspection-cli`: a new requirement for `exa-tool races`.

## Impact

- `exa_formats` (`races.rs`: `RaceBody`, `Gait`, `Race::body`,
  `body_of_character`; a dev-dependency on `exa_rpk` for the corpus test)
- `exa_tool` (`races` command)
- `docs/formats/OTHER.md` (Races), `docs/formats/RSQ.md` (Physical body),
  `docs/PLAN.md` (Phase 3), `crates/exa_formats/README.md`,
  `crates/exa_tool/README.md`
