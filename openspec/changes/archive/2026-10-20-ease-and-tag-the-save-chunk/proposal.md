# Proposal

## Why

The save settings chunk writes a header and one of two tags then two dwords, reads them back with an old-version flag, and uses a quadratic easing curve. Exarust has no save-chunk reader or easing curve.

## What Changes

- Easing returns `(2x)² × 0.5` below 0.5 and `((2x − 3)(2x − 1) − 1) × −0.5` otherwise.
- Writing uses the header `0xECADCA01`, a small or large tag and the two dwords.
- Reading accepts only that header, sets the old flag when 1,000,000 is above the tag float, and stores the dwords.

## Capabilities

### New Capabilities

- `save-games`: the settings chunk and the easing curve.

### Modified Capabilities

## Impact

- `exa_formats::save_chunk`, from `ease_save_curve`, `write_save_settings` and `read_save_settings`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`: `XA_SaveGames` moves from 4% to 5%.
- Evidence: the 90-byte easing curve with its `0.5`, `2.0`, `3.0`, `1.0` and `−0.5` constants. The 140-byte writer and the 104-byte reader: the header `0xecadca01`, the two tags, the old flag against the `1,000,000` constant, and the two dwords. No new corpus files.
- Out of scope: the stream routines.

## Phase

Phase 6, game modes. The save settings chunk and the easing curve.
