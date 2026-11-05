## Context

Exarust's held item is its own `Ragdoll` tied to the hand by the cross-body
grip stage (`exa_physics::grip_solve`). The original rebuilds the item's
place from the grip table on a re-place; Exarust's equip and torch already
place the item by that table (and the shield by its attach frame) when they
spawn it, and the grip builder measures the item's root part in the holding
hand's frame when it builds the links.

## Decisions

- The re-place keeps the attach's measured place (`HeldRest`) instead of
  recomputing the table: it is the same frame, and it covers the weapon,
  the shield and the torch with one rule. A flip since the build turns it
  half a turn about the item root part's Y axis, as the table's flipped
  placement does.
- Every part of a multi-part item is moved by the same rigid move as its
  root part, then placed at rest (`Ragdoll::place_at`), so the item has no
  velocity and no strain.
- The second-hand slack reset is gated as the original's: an armed grip
  class (0x30 bits), the link not the item's first, and the link enabled.
  Exarust reads the item's own class where the original reads the
  character's weapon class word (the same word for a single held weapon).
- The request is a marker component; the system runs in the grip build set,
  so a placement in the pre-update or the behaviour stage is re-placed
  before the next solver.
- The sole's kept axis starts at zero, as the original's zero-initialised
  part does; a moving body whose foot touches before any turn has taken an
  axis is not turned that iteration.

## Open questions

- The torch's re-place frame in the original is (13, −1.5, 6) cm and
  (70°, 15°, −20°), its attach routine's own; Exarust's torch is attached
  by the grip table's entries (−12, −1.5, 6) / (70°, −15°, 20°). The re-place
  follows Exarust's attach; the attach frame is a separate check (REA, the
  torch attach routine).

## Formats

No format page changes (no data read).
