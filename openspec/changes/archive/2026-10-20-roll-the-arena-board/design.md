# Design

## Behaviour

An instance copies its template and sets the rank: unranked stays −1, flag 2 takes the slot, otherwise `round(slot·(1−r⁴))` with the MT unit draw. The window is 0–5 for Challenger and slot..=rank otherwise. A flag-4 template draws a weapon row `rng.below(tier_len)+1`; secondary and cash scale by the slot.

The daily board draws `RANKED` seven ways with a cap/min-slot/reject loop and rolls each row. The tournament board draws `TOURNAMENTS` five ways, forces min 5 / max slot / slot 5 / rank row, draws the weekly prize rejecting the eight history indices and then shuffles 19 companies into 16 slots on the same LCG.

## Model

`exa_game::arena` adds the instance type and the generators.

## Open questions

The MT19937 state/seeding, the roster/entrant flow and opponent spawn.

## Docs

`XA_Arena` is **24%**. The headline rises.
