# Design

## Approach

Every rule was re-read in the instruction listing wherever a float, a
shift, a comparison or an index decides the result: the point conversions
and thumbs use `CVTSS2SI` (ties to even, a NaN gives 0 in the low half);
the centring divides truncate (a sign fix and `SAR`) except the unset y,
which shifts; the counter's back blocks step one pixel because the loop
adds one to the x; the eased fade keeps its value in a register no call
reads; the child raise's lower-layer search begins at index 1 while the
top-level raise's begins at 0. Constructor fields were matched by offset
(the 0.25 float sits beside the vertical fraction, the 0.025 one beside the
horizontal), and the attribute word's scrollbar bits by which client edge
they move. Tags were matched by the routines that read the forms' fields
(the inventory's constructor stores 0xce1c2010; the loot window search
compares its character field). The record and form fields became plain
inputs: a hit node, a frame, a sibling list, a control tree.

Removed: copies of `hud`'s layout, frames, shadow alpha and texture names;
identity functions that encoded nothing; the journal key mask (the
inventory's close rule, already in the item windows); the loot window's
size, the selector's tag and size guesses (the selector's own form gives
332 × 112).

## Open questions

- Who writes the inventory's and journal's remembered places: outside these
  units; the closing forms' handlers, to be read when their classes are
  (REA on the inventory and journal forms).
- The loot window's constructor and size: its class's unit (REA).
- The skill-ready alert's meaning (a level to spend) and the status
  entries' flags: the player-record and status units.
- The requirement table the technique selector reads: its data block, to be
  read with `read_bytes` when the selector's rows are drawn.
