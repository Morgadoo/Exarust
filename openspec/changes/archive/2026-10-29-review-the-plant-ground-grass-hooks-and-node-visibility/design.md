# Design

- `ground_with_normal` takes the block's vertex positions in centimetres
  and its row length `n` (241 for a retail block) instead of a fixed stride;
  a cell whose neighbours fall outside the grid returns the position's
  height (labelled guess: the original reads on into the next row).
- The three hook predicates take the words as plain integers so a future
  renderer path can call them per chunk; nothing calls them today, so frame
  time is unchanged.
- `apply_visibility_flags` rebuilds child lists from the stored 1-based
  parent indices and walks them in node order, bounded by the node count so
  a cyclic parent list cannot hang. It writes `flags2` bit `0x2`; no system
  reads that bit yet.

Nothing in the running game changes.
