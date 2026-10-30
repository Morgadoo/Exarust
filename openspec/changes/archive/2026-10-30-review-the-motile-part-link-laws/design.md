## Context

Exarust's held item is a separate `Ragdoll` tied to its holder by a
cross-body grip stage; the original's is a child of the hand in the
character's part tree. The original's part iteration runs the living law
only for a part whose tree root lives and whose own word carries the
living-part bit, which only the character's own skeleton parts with muscle
anchors receive. Every other part, a held item included, runs the base
iteration.

## Decisions

- The grip stage picks the law by the link's pivot flag, not by the
  holder's drive: a pivot link takes `limp_law` (0.001 softening per cm²),
  a link without it takes `body::plain_law`. `plain_law` is the tail of
  `WorldLink::correction`, factored out so the two share one
  implementation (gap and slack in cm, the taut length reported).
- The living limit law stays as it was before the lane (the literal
  compare of the cm² gap with the stored linear slack).
- The held-weld writes at collapse and death are removed rather than kept
  as unused functions: the routines that seemed to make them write the
  legs' limit links, which the body already models.

## Risks

- Runtime behaviour returns to main's for held weapons (the lane's living
  hand law never shipped); world welds stay disabled in the game, so the
  plain law changes nothing visible yet.
- A corpse's shield or torch no longer gets a stiffer soft weld; since the
  welds are disabled, nothing in the game changes.
