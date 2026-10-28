# Design

- `grid::special_tile` turns every table entry by the cell's rotation; a
  type past the table keeps no mask and its floor (the original reads
  unrelated data there; no shipped level uses one).
- `Rooms::flood` removes self links with `swap_remove` in a forward walk
  that re-tests the moved link, and runs `group_links` over every room
  after the last room is pushed.
- `Rooms::door_reach` compares the two cells' rooms as options, so two
  `None` rooms reach; otherwise it finds the first link into the second
  cell and asks `passable` of that link only.
- `RoomTracker::is_active` reads only the fade (`Active` or `Reentering`).
  The original's reset leaves the working state alone; Exarust's
  `show_all`/`hide_all` clear it, which is documented.
- `RoomLights::propagate` tests whether any listed light's home is the
  room.
- `grid::ray_blocks(shells, active, origin, dir, max_t, lid)` takes an
  activity predicate (the tracker's `is_active` in the game) so it can be
  tested without driving fades; `segment_blocks` builds the direction as
  `d · (1 / |d|)` and the limit `|d|`. `Strip::ray_hits` sums dot products
  as `(x + y) + z`, as the instructions do.
