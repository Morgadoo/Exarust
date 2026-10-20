# Design

## Behaviour

A tier's weapon row is `rng.below(tier_len)+1`; the variant is the non-zero draw from the row, with the row passed 0-based as `set_weapon` receives it. A company opponent's level is `60000 + rng.below(9999)`; a generic opponent's is `50000 + rng.below(9999)`.

## Model

`exa_game::arena` adds the three helpers.

## Open questions

The actor/body creation, condition and equipment costs.

## Docs

`XA_Arena` is **30%**. The headline rises slightly.
