# Design

The record is held in the running game as `Possessions`, which the level
spawn already builds for placed characters and corpses. The loot window's
panels are rebuilt from it each time the window opens, so moving entries
between `Possessions` keeps the display right without a record write-back.

Icons: the original acquires an icon by rendering the item. Until Exarust
has that renderer, `PanelIcons::reserve` hands out a slot with the panel
icon's half extents and no atlas cell, which the draw skips; placement,
clamps and drags use the reserved halves.

Equipment: the original lists worn items, then each of the two hand pairs
whose item has a shield section. Exarust asks the item catalogue; a
level-local id is not in it and is left out (deviation until promotion).
The scatter draws from the engine's shared generator in the original; the
panel starts its own generator from the run-time seed the original stores,
a deterministic stand-in.

Flag words: the show rule's word is the record's profession word, which
`Possessions.flags` keeps. The deferred close's state mark has no Exarust
copy and reads 0; its condition half reads `exa_combat::Condition`.

Drops onto the equipment panel and outside every panel are refused, since
the character view and the world drop are not wired.
