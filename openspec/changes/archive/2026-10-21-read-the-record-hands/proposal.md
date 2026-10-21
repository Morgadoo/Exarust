# Proposal

## Why

The character record reader dropped each held item's flags word and kept
only the sex and race of the profession word, so a character's possessions
started with no flipped weapon, no lit light and the first weapon set out.

## What Changes

- Keep the four held items' flags words (`CharacterRecord::hand_flags`)
  and the whole profession word (`CharacterRecord::profession`).
- Start a character's possessions from them.

## Capabilities

### New Capabilities

### Modified Capabilities

- `carried-items`: possessions from the record.

## Impact

- `exa_formats::characters`, `exa_items::possession::Possessions::of_record`.
- `docs/formats/RFC.md` (character records).
- Evidence: REA on the weapon-set swap (it toggles bit `0x0100_0000` of the
  word the sex and race are read from) and on the hand plan (flags
  `0x0100_0000` flipped, `0x0200_0000` lit).
- Out of scope: applying the flip to the grip.

## Phase

Phase 4 (items), `docs/PLAN.md` section 7 item 2.
