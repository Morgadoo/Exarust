# Design

## Behaviour

A row holds a count, two entrants and a team id. A company profile starts `[3, tier, 100, 10, 10, 0, 0, 0]`, sets bit 2 when armed and bit 3 plus the company's three memory-order bytes when a company is given. Pending rows below 0xCCC condition lose entrants by shifting; empty pending rows take a 2..6 team id until the first live row.

Resolution tallies the six rows by team id. A player tally of 3+ without a tie wins: the weekly prize is granted, championships and the week flag rise, and on week 3 with all four flags the set at `(day % 28) % 4` is granted and the season counter rises. A company win stores its shuffled company id+2. The non-champion path counts the day when any row has entrants and clears the week flags on week 3.

## Model

`exa_game::arena` adds the roster, profile, filter and resolution types.

## Open questions

The MT19937 seeding, the live board UI, the actor spawn/body calls and the shop.

## Docs

`XA_Arena` is **28%**. The headline rises.
