# Design

## Behaviour

Each tier is a fixed list of weapon rows. A row has a name and one to three variant ids; the variants are filtered to the non-zero ones and one is drawn with the engine generator (`state = state·0x8088405 + 1`, the high half of `state·count`). Tier 5 repeats tier 4's four polearms. The point limits are 5, 10, 15, 20, 30 and 40.

## Model

`exa_game::arena` adds `WeaponEntry`, `WEAPON_TIERS`, `weapon_tier`, `pick_variant` and `point_limit`.

## Open questions

The loadout cost, the opponent spawn, the live board and the shop are separate.

## Docs

`XA_Arena` is **20%**. The headline rises slightly.
