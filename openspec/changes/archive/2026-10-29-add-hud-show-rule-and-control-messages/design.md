# Design

The HUD draw keeps its own fade state and stored values; only the show
test moved into `game_gui::bars_shown`, which the draw calls with the
yellow health, focus, combat mode, the hold clock and `power_bar: false`.
The clocks are passed as the original's 32-bit unsigned game clock (the
virtual clock in ms truncated to `u32`), so the hold compares unsigned as
the instruction does. The draw's cost is unchanged: a handful of compares
per frame and no new allocation; the painted image is still rebuilt only
when its key changes.

The remaining rules are plain functions over values: nothing in the
running game has a live counter, bar control, technique selector or
deferred fill yet, so they are tested with the original's numbers.

The GUI job queue's runner removes a job that answers false and keeps one
that answers true; the deferred fill answers true (retry) while the
character generator has no node and fills the inventory, answering false,
once it has one.
