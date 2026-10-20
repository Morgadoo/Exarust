# Design

## Behaviour

`Mt19937` uses the standard 5489-style initialisation: state[0] = seed, state[i] = 1812433253·(state[i−1] ^ (state[i−1]>>30)) + i. The twist combines the top bit of one word with the low 31 of the next, XORs the 397-ahead word and the `0x9908B0DF` conditional. Tempering uses `>>11`, `(<<7)&0x9D2C5680`, `(<<15)&0xEFC60000` and `>>18`. The unit draw multiplies by 2⁻³².

`roll_instance_mt` draws a unit only when `needs_rank` is true; a flag-2 or unranked template consumes nothing.

## Model

`exa_game::arena` adds `Mt19937` and `roll_instance_mt`.

## Open questions

The arena's MT seeding and the other consumers of the shared stream.

## Docs

`XA_Arena` is **29%**. The headline rises slightly.
