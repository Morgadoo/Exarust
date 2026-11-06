## Why

The owner reported that characters do not hold their weapons well. Since
round 85, in `play arena1 --opponent --weapon ub05ssworda`, the player's
sword hung vertically in front of the hips while the right hand was empty
at the side. Round 84 (252e8c8f9) held it. Round 85's first commit
(7f1cd24ab) still held it; the regression came with cebb3cb29, which builds
a body behind the loading screen only once the body is dressed.

## Evidence

Traces of the held item and its hand on each tick, and the ground
placement's logs, comparing 7f1cd24ab with the lane:
- 7f1cd24ab: the sword's body was registered from a hand node the body had
  not drawn yet, tilted a few degrees off its grip. The player's ground
  placement found "0 probes on a body mesh".
- After cebb3cb29: the sword is built in its true grip, its blade over a
  foot. The ground placement's Height Query took the player's own sword
  for ground: "1 probes on a body mesh above the level … character ground
  42.6". The body was put 42 cm up and dropped. The fall wrenched the
  sword: it ended 20 cm off the hand, hanging vertically, and stayed there.
- The ground query was always able to do this. Round 84 missed it only
  because its sword was registered at a slightly wrong angle.

Further gaps of the dress-first order, found on the way:
- A grip whose holder had no body yet carried a placeholder holder key
  until `resolve_pending_grips` ran. That system was unordered against the
  grip builder, so the builder could tie the links first, and the release
  that follows it then dropped the item as having no owner. A unit test
  through both plugins shows the drop.
- The item's body was built in the frame after the equip, at a hand node
  that the not-yet-built body had never placed.
- The merged character mesh runs carry a body part of bone 0. A body built
  after its dressing took them as two extra free root parts: the player
  had 20 parts and its hand was at index 19 instead of 16.
- The torch was given no body at all when lit before its holder's body
  existed.

REA, the Height Query: its mesh mode tests static meshes and dynamic
bodies. It skips a body out of the world (`0x800`), a living character's
part (family `0x12000000`, unless the character's state is dead `0xA00`)
and a placed rigid body (family `0x10000000` with the placed bit
`0x1000`). Nothing in it leaves out a character's held items. [inf]
Leaving them out of their own holder's placement is Exarust's own rule:
the placement moves them with the body anyway (the re-place puts them back
in the hand right after).

## What Changes

- exa_physics: `build_grips` waits while the holder key is still the
  placeholder. `register_props` builds a held item's body only once its
  holder's body exists. Debug logs record a grip being tied and let go.
- exa_character: `resolve_pending_grips` runs before the grip builder,
  then `release_collapsed`.
- exa_game: `build_bodies` leaves out the merged mesh runs' parts. The
  ground placement leaves out the items the placed body holds. A torch lit
  before its holder's body is built gets its body and a pending grip.

## Out of scope

- In combat stance the sword still sits loosely in the hand during the
  stance change. Round 84 does the same, so it is not part of this
  regression.
- In `play arena1 --opponent --weapon …` the opponent's trace differs
  between two runs from tick 5. Round 85's build (5f5e7a1ab) differs the
  same way. `play arena --opponent` and exanima01 repeat.
