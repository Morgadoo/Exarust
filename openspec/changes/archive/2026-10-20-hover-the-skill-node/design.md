# Design

## Behaviour

When the index and the current value are both non-positive, the base `0x10002d6d0` runs and its result is returned. Otherwise the result is 3 and `0x6c` stores the index. Without a grid pointer at `0x170` the method ends there.

With a grid, the candidate is the index only when it is positive, its bits 8 through 10 are non-zero, its low three bits are non-zero and the byte at `0xb8 + row × 8 + column + 1` is non-zero, where row is bits 8 through 10 and column the low three bits. A candidate equal to the previous index at `0x18c` ends the method.

A candidate that is positive, differs from a positive last positive at `0x190`, stamps the clock at `0x194`. Every other new candidate stores the elapsed `clock − stamp`; when that elapsed is under 200 the stored stamp becomes `clock − (200 − elapsed)`, otherwise the clock. The candidate becomes previous, and a positive candidate also becomes last positive.

## Model

`skill_hover` takes the index, current, grid, previous, last positive, stamp, clock and base result, and returns the result with the updated state. A missing grid byte is treated as unset.

## Open questions

The under-200 correction's purpose is not established; it is reproduced as read.

## Docs

This method is 287 bytes. `TSkillUI` moves from 7% to about 9%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
