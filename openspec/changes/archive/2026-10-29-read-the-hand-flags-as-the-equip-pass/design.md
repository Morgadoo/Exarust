## Decisions

- One helper (`offhand::insert_equipped`) for both spawn paths, so the arena's fighters and the level's characters attach the same way.
- The dead keep the moveset's hands and hang no off-hand item: the corpse spawn equips without the attach pass.
- The off-hand torch is not hung: the equip pass leaves it unflipped (unlit), and Exarust's carried torch always burns.
