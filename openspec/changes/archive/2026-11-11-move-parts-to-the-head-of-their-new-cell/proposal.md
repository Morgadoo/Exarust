## Why

Phase 3 in docs/PLAN.md: the remaining gaps of TSimSector (round 156: the head-of-cell move on cell migration, the
16-bit list index, a tree's parts stepped together, the free-thread insertion order) and of TTetraRBD (round 137:
the velocity writers' third-set handling, the clone's run-time caller and node-frame rewrite).

## What the original does

REA and `exe.asm` of the grid sector's step, the pair key writer, the rigid body's Verlet step, its post-step and
every routine that writes a body's particle sets:

- **Cell migration.** After the 20 iterations the step walks the solver's awake list from its start; for each part it
  runs the inverted-tetra repair, rebuilds the matrix and the mesh-box centre, takes its 6 m cell and, when the cell
  changed, unlinks the part from its old cell's list and links it at the head of the new one (reporting a duplicate
  link). Sleeping parts are not walked. The detection follows at once. A cell's list is therefore its parts by the
  time they last entered it, the latest at the head (joining links at the head too).
- **The key word.** The appender writes the list index into the 16-bit word under the 16-bit order key; the purge
  renumbers keeping the order. Ordering by Exarust's place gives the same comparisons until a sector holds 65,536
  parts.
- **The step order.** The step calls each awake part's own Verlet step in the list's reverse order. A part's step reads
  its own particle sets and its character's words (the carry, written by the character code; the tone); the tree's
  root, last of its tree, restores the whole tree's edges. The carry propagation is called only from the character
  code. So stepping a tree's parts together gives the same values.
- **Insertion.** Nodes added during play are queued by the scene's add hook and appended at the next upkeep in queue
  order, before that tick's step: the order Exarust's insert hook gives them.
- **Particle sets.** The Verlet step's velocity is the previous set less the third;
  between ticks the two are equal. Every particle writer outside the step and post-step (the
  freeze-at-matrix, the place-at-node routines, the held items' re-placing, the character placements) writes the
  current set and copies it into the previous and third sets alike. Exarust keeps the third set's role in two
  fields (`previous` and `two_ticks_ago`); its velocity setter wrote only the first, so the two-step point missed
  the velocity.
- **The clone.** Its callers are the four body classes' clone slots, reached from the node clone that copies a
  library model into the scene; Exarust builds each body from the model's data instead, with the same values
  (round 40), so `Ragdoll::runtime_clone` keeps no run-time caller.

## What Changes

- `body::pairs::CellEntry` on each `Part` (the cell it was last linked into and when); the pair stage's `link_cells`
  joins new bodies in list order and moves each awake part whose cell changed to the head, in awake-list order, on its
  own clock before the detection; each cell's parts are met latest-linked first.
- `Ragdoll::set_velocity` also writes the two-ticks-ago particles.

## Out of scope

- A sleeping part moved by code (a lift, a teleport) is bucketed by where it is; the original leaves it in its old
  cell until it wakes. Its entry is kept, as the original's, and it moves to the head when it wakes.
- The 16-bit wrap of the list index (65,536 parts in one sector) and the clone's run-time caller (none in Exarust).
