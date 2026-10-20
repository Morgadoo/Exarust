# Design

## Behaviour

The rank roll computes `slot·(1−r⁴)` and converts with round-to-nearest-even. At slot 3 and r 0.5 the value is 2.8125, which rounds to 3, so the Elimination window is 3–3. The company table's three build bytes are read at record offsets +8, +9 and +10; Fitten Gang is therefore `[0x42, 0x8C, 0x01]`.

## Model

`roll_rank` uses `f64::round_ties_even`; `COMPANIES` stores the bytes in memory order.

## Open questions

The MT19937 rank stream and the board generators are separate.

## Docs

No coverage change.
