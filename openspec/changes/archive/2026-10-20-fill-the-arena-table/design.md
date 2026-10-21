# Design

## Behaviour

The templates are copied from the binary's three back-to-back 0x70-byte arrays. Cash is a three-byte base-12 packed value: `b0 + 12·b1 + 144·b2`. A match at slot `s` pays `pack(unpack(cash)·(s+1))`; its XP pot is `trunc(xp·(1 + 0.25·s))`. The pot is then scaled by `hp_left/hp_max`, truncated, and increased by half when the company's manager fights; each fighter receives `trunc(total·damage/total_damage)`.

Rank points are one plus the sum of the six pools divided by 5000. The loadout tier is `min((rank−1)/2, 5)`; one award gives `min(total/rank, 5000)`. Wages are `rank−1` for the 0x30000000 class, 2 for 0x40000000, 12 for 0x50000000 and `tier+1` otherwise. Recovery saturates at 0xFFFF; a fighter at or above 0xCCC is fit.

Day 1 mod 7 starts a week, day 0 ends it; the season is four weeks. The prize pools and companies are the binary's tables.

## Model

`exa_game::arena` exposes the const tables and the pure formulas.

## Open questions

The match flow, opponent generation, live board, shop and tournament resolution are separate. The template's last value is stored as `secondary` and is not called a prize.

## Docs

`XA_Arena` is **15%**. The headline rises.
