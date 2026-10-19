# Design

## Context

The fee scale already accepts a unit. That unit is a tempered word divided by 2^32. The record copier walks fields, copying raw gaps with a byte move and then each field. Managed kinds are a separate path.

## Goals / Non-Goals

**Goals:**

- `state[0] = seed`. Later words are `i + (prev xor prev >> 30) * 0x6c078965`. The index starts at 624.
- A draw at 624 twists, tempers word 0, and leaves the index at 1.
- Twist: upper bit of this word, lower 31 of the next, shift once, xor `0x9908b0df` when the next word is odd, xor the word 397 ahead.
- Temper: xor shift 11, xor shift-left 7 masked by `0x9d2c5680`, xor shift-left 15 masked by `0xefc60000`, xor shift 18.
- The unit is that word times the stored scale `0x3df0000000000000`.
- If the mark differs from the stamp, or the index is at least 625, seed from the mark, reverse the mark, and copy it onto the stamp.
- A record copy writes each gap, each plain field, and the tail. A short buffer or a field out of order or past the size returns an error.

**Non-Goals:**

- String, dynamic-array, and variant assignments.
- Moving `XA_Arena` for this. These routines sit in the runtime, which stays at 4%.

## Decisions

1. `UnitGen::seed` starts with matching mark and stamp, so the first draw twists the seeded words.
2. `copy_record` takes plain offsets and widths. The test uses synthetic bytes.
3. A daily scale test calls `next_unit` and passes that value to `place_daily_match`.

## Risks / Trade-offs

- `XA_Arena` stays 32%. The runtime section stays 4%. The game-modes section stays 0%. The headline stays 14.4% and 5.7%.

## Migration Plan

- New functions beside the fee scale. Existing draws are unchanged.

## Open Questions

- None for the seeded words and the plain-field copy.
