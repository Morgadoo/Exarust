# Design

The marker search takes an `overlaps(frame)` callback. The live lift
moves the held body's parts so the held part sits at the marker's frame
(`item_cursor::snapped_parts`) and tests each part's spheres against the
world spheres of every other body (the player's body is not in that
query). Touching counts as overlap; the original's narrow phase decides
contact itself, so the boundary is Exarust's choice. The other bodies'
spheres are gathered on the first call in a frame and reused for the
next markers, so a lift with no matching marker does no extra work.

The use cursor and the deferred drop are kept as pure functions
(`player_controls::{use_begin, use_end, run_spawn}`) returning what the
original writes; the right press and the GUI drop do not call them yet.
