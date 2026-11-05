# Design

Extends `docs/formats/RSQ.md` (Physical body: the character frame, support)
and `docs/PLAYER.md` (Movement).

## The character's place

Exarust keeps its model root at the feet (the original's frame sits at the
pelvis's height). The character entity's place is therefore:

- across the ground, the frame's origin less the body root's offset from
  the character (the carry stage writes it as the follow point);
- in height, the character's ground the feet routine measured that tick
  (`Ragdoll::character_ground`), the ground the original keeps for the
  character. Without support the place goes down with the pelvis's own
  drop this tick, never under that ground, so a fall is not shown early.
  A ground more than 3 m under the pelvis, or above it, is ignored
  (Exarust's guard: a far ground under a fall, or one not measured yet).

The move runs at the start of the fixed step, where the capsule's ran, so
the character ticks read where their characters stand. It is a single
assignment per character: no query except, for a character without
muscles, one ray down.

## Open questions and how they were answered

- Does any per-tick routine keep the character position out of walls? REA:
  every writer of the position listed in the proposal, each read in the
  assembly; the per-tick grid snap is the base class's slot (returns
  false), the pathfinder's VMT entry read from the image.
- Is the "standing without support" balance reset unconditional? The feet
  routine's assembly: a locomoting test, a support test, then the store of
  0, before the get-up's forced support (S = 0.5 × that balance).

## Risks

- Bodies that relied on the perched exception (a capsule on rubble with
  both ankles 35 to 44 cm up, exanima01's start room) now lose their
  balance there, as the original's do.
- A get-up with no foot grounded has no support until a foot finds ground
  (the original's order).
