# Design

## Behaviour

`TWeaponBlood` is a dynamic array at `0x10` with a count at `0x8`; each spot is 24 bytes: a source pointer at 0, an amount at `0x8`, a scale at `0xc`, an inverse at `0x10` and a spare word at `0x14`. Adding a source first refuses a source whose flags carry `0x400000`. It scans the live spots from the first for a zero source; when none is free it raises the count and takes the last slot. The scale is the sum of the source's floats at `0x14` and `0x34` widened to double, times the double `0.0125`, narrowed again. The inverse is the single-precision square root of `80` over that sum. The amount and the spare word start at 0, and the return is the index plus one. Clearing the count sets it to 0 and keeps the array. Clearing a spot at `edx` writes the slot at `(edx − 1) × 0x18` and zeroes the source, amount and scale. Adding to the amount stores the sum clamped between 0 and 1.

## Model

`WeaponBlood::add(source, length_a, length_b, hidden) -> usize`, `clear_count`, `clear(index)` and `add_amount(index, delta)` on a `Vec<WeaponBloodSpot>`. A hidden add returns 0; indices are the original's 1-based ones.

## Open questions

The spot's spare word and the draw are not established; the draw is out of scope.

## Docs

`TWeaponBlood` is **53%** (453 of 853 bytes). The combat section is **14%** reimplemented. The headline stays 14.4% and 5.7%; the fresh sum stays about 15.5%.
