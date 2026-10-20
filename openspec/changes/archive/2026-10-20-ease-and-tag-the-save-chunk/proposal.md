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
- Evidence: the 90-byte `0x10017d160`: `0.5` at `0x10031d958`, `2.0` at `0x10031d960`, `3.0` at `0x10031d970`, `1.0` at `0x10031d978` and `−0.5` at `0x10031d968`. The 140-byte `0x10017d030` and the 104-byte `0x10017d0c0`: the header `0xecadca01`, the tags `0x2c7ba0` and `0x2c7b90`, the `0x4f2d20` old flag against `1,000,000` at `0x10031d950`, and the two dwords. No new corpus files.
- Out of scope: the stream routines.

## Phase

Phase 6, game modes. The save settings chunk and the easing curve.
