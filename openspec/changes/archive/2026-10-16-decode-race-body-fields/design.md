## Context

A character's body reads its race record through one pointer, set when the
character is built. The builder derives the race id from the character
record's race byte plus one and looks it up in the `races.rdb` id database.
That database is loaded raw (index and record blob copied as they are), so
the pointer is into the file's own record, and every read of a race field
uses the file's own offset. A built-in record (named "human", every factor
1.0, an all-zero gait table) is used only when the id is missing or its
record is empty. The loader also derives a per-tick gait table from the
record's table at `+0x94`. The research notes, with addresses, are
git-ignored: `context/re/body-drive.md` (§2, §5, §7, §10, §12),
`body-links.md` (§4.3), `body-feet.md` (§3), `body-tick-order.md` (§4.2),
`player-control.md` (§1.4) and `body-controller-plan.md` (C2 and the
answer to Q2).

The format page this change extends is
[`docs/formats/OTHER.md`](../../../docs/formats/OTHER.md) (Races).

## Goals / Non-Goals

**Goals:**
- the five factors and the gait table decoded for every shipped race, with
  their meaning and how each was established;
- the original's fallback for an unknown race;
- `exa-tool races` to print them.

**Non-Goals:**
- consuming the values (later changes of the plan);
- decoding the rest of the record.

## Decisions

- **Plain values, file units.** `RaceBody` keeps the file's floats as they
  are: factors as stored, gait thrust and clip rate per second, the push
  scale per second². The original's loader divides thrust and clip rate by
  83 and the push scale by 83² to get per-tick values; that conversion
  belongs to the consumer (C18), which runs at 83 Hz. The doc comments
  state the scaling.
- **Column 2 is not decoded.** The loader never reads the record's third
  gait column (0 in every record). It fills that slot with a built-in value
  per row (0.05, 0.2, 0.25, 0.05), whose meaning is not traced. `Gait`
  therefore has four fields; the built-in values are documented, not
  decoded.
- **Fallback.** `RaceBody::FALLBACK` holds the built-in record's values.
  `body_of_character(races, byte)` returns the race's body or the fallback,
  as the original's lookup does. `of_character` is unchanged.
- **Corpus test in `exa_formats`.** It needs to read `Resource.rpk`, so
  `exa_formats` gains `exa_rpk` as a dev-dependency only (neither depends on
  Bevy, and `exa_rpk` does not depend on `exa_formats`). The test runs
  under `cargo test` when `EXANIMA_DIR` is set and returns early when it is
  not.

## Open questions

| question | how it is answered |
|---|---|
| Which record the body reads (Q2) | Answered by REA on the race lookup and loader: the loaded file record. |
| What column 3 (the gait clip rate) feeds | Not needed here; C10 and C18 trace the clip clock (Q7, Q4). Labelled as inferred in the docs. |
| What the built-in column 2 value does | Not needed here; left for C18 (locomotion), labelled unknown. |
| The other floats from `+0x58` to `+0x90` | Not needed by the plan's changes; left undecoded. |

## Risks / Trade-offs

- [Some meanings are inferred from a single consumer] → each field's doc
  and the format page say how it was established and label inferences.
- [A malformed record] → records shorter than 624 bytes are already
  rejected; the new fields lie inside that size, so parsing stays total.
