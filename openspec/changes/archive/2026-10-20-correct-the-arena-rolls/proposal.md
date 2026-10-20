# Proposal

## Why

A fresh decode of `0x100183f80` shows the arena rank roll uses `cvtsd2si` (round-to-nearest-even), not truncation, and the company profile bytes are consumed in memory order `[+8,+9,+10]`. The existing model truncates and stores the bytes reversed.

## What Changes

- The rank roll rounds to nearest even.
- The company profile triples are stored in memory order.
- The Elimination test changes from rank 2 to rank 3 at slot 3, r 0.5.
- A test locks Fitten Gang at `[0x42, 0x8C, 0x01]`.

## Capabilities

### New Capabilities

### Modified Capabilities

- `arena-career`: the rank roll's rounding and the company byte order.

## Impact

- `exa_game::arena`, from `roll_rank`, `rank_window`, `COMPANIES` and their tests.
- `docs/PLAN.md`; the coverage headline is unchanged (a correction to existing credit).
- Evidence: `0x100183fcf` (`cvtsd2si`), `0x1002bbf30` (the 19 profile records, read byte-for-byte). No new corpus files.
- Out of scope: the separate MT19937 rank stream and the board generators.

## Phase

Phase 7, arena. The rank and company corrections.
