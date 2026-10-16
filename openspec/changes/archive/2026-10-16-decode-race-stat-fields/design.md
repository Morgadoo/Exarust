## Context

C2 (`decode-race-body-fields`) decoded the five body-controller factors and
the gait table; L6 added the first flag word (bit 0, bleeding). The master
plan (`context/re/master-plan.md`, F3 and its Critic, K.3.4 and K.4) makes
this change the only one that edits `races.rs`, so every remaining field a
planned change reads is decoded here: lane B's clip clock (M14) and steps
(M16), lane C's weapon record (M21) and grips (L12), lane F's derived stats
(M41) and regeneration (M43) then only read them.

The original reads every race field through one pointer to the loaded
record, with the file's own offsets. When the race id is missing it uses a
built-in record that goes through the same loader. The research notes, with
addresses, are git-ignored: `context/re/master-body.md` §7.2–7.4 (with this
change's correction appended), `master-animation.md` §3.1 and §8.1,
`master-movement.md` §5.1 and §6.2, `combat-damage.md` §1, `master-combat.md`
§2.3–2.4 and the grip placement.

The format page this change extends is
[`docs/formats/OTHER.md`](../../../docs/formats/OTHER.md) (Races).

## Goals / Non-Goals

**Goals:**
- every race field a planned change reads, decoded for all 18 shipped
  races, each with its meaning and how it was established;
- the original's built-in fallback for each;
- the gait column the loader fills in, and the feral sprint blend, so the
  locomotion and spawn changes read one rule;
- `exa-tool races` to print them.

**Non-Goals:**
- deriving per-character values (scale with height, reach with feral and
  encumbrance, rule bits from race bytes and the record): M41;
- consuming any value in the simulation;
- the fields no planned change reads (proposal, Out of scope).

## Decisions

- **Two groups.** The clip-rate base, size, scale and mass divisor join
  `RaceBody`: they are body scalars with the same fallback (1.0) and the
  same lookup (`body_of_character`). The rest go into a new `RaceStats`
  (`Race::stats`, `stats_of_character`), which also holds the second flag
  word and the kind word. `RaceStats` holds the footstep set name, so it is
  `Clone`, not `Copy`; its fallback is still a constant.
- **Units.** Lengths become metres, as the other parsers return them: the
  unarmed reach (stored in cm), the held offset (signed bytes of 0.1 cm),
  the brake distances (the loader's 0.05/0.2/0.25/0.05, read × 1000 as cm).
  Factors, the clip-rate base and the gait rates stay as stored; the
  per-tick conversion of the gait rates stays with their consumer, as C2
  decided.
- **The scale cap is a constant, not applied.** The character builder caps
  the scale at 1.5 before the height factor (the harvester and the demon
  store 1.8). The parser keeps the stored value and exports `SCALE_CAP`, so
  M41 applies the original's order (cap, then height) in one place.
- **Brake distance in `Gait`.** The loader never reads the record's column
  2; it writes its own value per row, for every race and the fallback. The
  parser does the same, so `Gait` describes the row the original uses.
- **Feral sprint.** `RaceBody::sprint_gait(feral)` returns the sprint row
  for a feral level of 0 or less (the original copies it), else every column
  moved linearly toward the jog row. The original blends the per-tick
  values; the per-second values are the same up to the loader's linear
  scaling. M41 calls it at spawn.
- **Flag bits are named only where a reader was traced.** First word:
  0x1 bleeds (L6), 0x8 may rise again in the story, 0x80 red regeneration,
  0x200 hovering feet. Second word: 0x4 red regeneration, 0x8 quadruped.
  Kind word: 0x100 claws. Other bits are documented with what is
  known (rule bits 1 and 4 from the second word, the item size bits
  0x10000/0x20000, the golem bits 0x1000/0x2000) and left unnamed.
  `Race::regenerates_red` and `Race::clawed` combine the words the way the
  init and the moveset chooser test them; the rest (story mode, feral
  level, race bytes) is M41's.
- **Fallback values** come from the built-in record in the image: every
  factor 1.0, unarmed reach 100 cm, swing turn and recovery 1, attack bonus
  0, perception 1, no held offset, an empty footstep set, voice volume 0 and
  attack voices 0.

## Open questions

| question | how it is answered |
|---|---|
| Which offsets each consumer reads | Answered by REA and `exe.asm` (every load through the character's race pointer and the builder's entry; constants read from the image). |
| Whether the "extra gait rows" of the plan are more rows in the file | Answered by the loader: the file has four rows; the extra row is the init's blend of the sprint and jog rows, and the extra column is the loader's brake distance. Recorded as a correction in the notes. |
| What race bits `0x2`/`0x4`/`0x10` and the second word's 1 and 2 do | Not needed by the plan's changes; left unnamed (RB49 covers the rule bits they set). |
| Whether a race's item size bits gate equipping | Not needed here; the item check is documented, its caller untraced. |
| What `+0xE8`–`+0x108` drive (read by view and camera routines) | Out of scope; listed in the format page as untraced. |

## Risks / Trade-offs

- [Some meanings rest on one reader] → each doc says so; the field values
  are corpus facts either way.
- [A malformed record] → records shorter than 624 bytes are already
  rejected, and every new field lies inside that size, so parsing stays
  total.
- [Editing lane C's `exa_combat` test] → one line filling the new field of
  a synthetic race, named in the proposal.
