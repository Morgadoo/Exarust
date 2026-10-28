# Proposal

## Why

TMotilePart was rated 54%. The equip/swap refresh measured the first held
item's class, reach and aim height, but not the aim point's weights on the
item's body or each item's own point. The weapon switch traded the blade
slots but not the hands the moveset is chosen from. The unarmed equip's
hand word had no rule. The motilepart8 lane adds them.

## What Changes

- The refresh weighs the first held item's aim point (0, 0.9 of its node
  extents, 0) on the item body's four rest particles with the engine's
  exact barycentric routine and keeps the weights on the character. With
  an item that is not a weapon first, every item's own point is weighted
  the same way; that is a tested function, since Exarust has no such item.
- The refresh's reach is now documented as an intermediate: the equip
  pass the refresh calls next clears the reach word and keeps the longest
  record reach, which the AI already publishes.
- The weapon switch also trades the holder's main and second weapon in
  `Hands`, so the moveset is chosen again from the new main weapon.
- The unarmed equip's hand mobility (half the base, a quarter with heavy
  claws) is a tested function.

## Phase

Phase 4, Combat and items: held items and their grips (docs/PLAN.md).
