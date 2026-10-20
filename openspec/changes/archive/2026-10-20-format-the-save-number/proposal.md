# Proposal

## Why

The save screens format a number as a zero-padded three-character decimal, using `"000"` for zero and negatives. Exarust has the chunk and version gate but not the formatter.

## What Changes

- Zero and negative values give `"000"`.
- Positive values write their decimal digits and pad on the left with zeros to three characters.
- Longer values are left whole.

## Capabilities

### New Capabilities

### Modified Capabilities

- `save-games`: the save number formatter.

## Impact

- `exa_formats::save_chunk`, from `format_save_number`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`: `XA_SaveGames` moves from 6% to 7%.
- Evidence: the 258-byte `0x10017daf0`: the zero branch to `"000"` at `0x10031da50`, the digit loop with `idiv 10`, the left prepensions of `"00"` at `0x10031da70` and `"0"` at `0x10031da90` while the length is under 2 and 3. No new corpus files.
- Out of scope: the string builder.

## Phase

Phase 6, game modes. The save number formatter.
