# Design

## Behaviour

A value counts when it is above zero as an unsigned word. The seed advances by multiplying with `0x08088405` and adding one, wrapping at 32 bits. The index is `(seed × count) >> 32`, which lies in `0..count`. The value at that index in the three-word table is returned, along with the new seed and the index.

## Model

`pick_arena_value(values: [i32; 3], seed: u32) -> ArenaPick { value, seed, index }`. The table is taken as given, so a positive entry after a zero is not moved.

## Open questions

The consumer call is not in.

## Docs

This method is 138 bytes. `XA_Arena` stays about 34%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
