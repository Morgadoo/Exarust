# Design

The placement sweep reuses `Physics::level_sweep` (a ball cast against
fixed colliders) with the held part's widest collision sphere, started
`min(hit, 6 m)` before the pick along the cursor ray. A zero time of
impact means the ball started overlapping the level; it is treated as no
contact so the item keeps the pick rather than jumping to the sweep start.
The cost is one shape cast a frame while an item is lifted, as before.

`item_cursor::sweep_to_contact` is the many-sphere form for a caller with
every sphere; it folds through the existing `drag_hit` so the strict-nearer
rule lives in one place.

The hover word keeps the earlier pass's constant names (renaming them would
touch the reviewed release rule); their meaning is documented on the
constants. `use_release` takes an explicit `UseOver` and `DoorLock` so the
dispatcher's GUI branches and the lock test are stated, not folded into one
target enum.
