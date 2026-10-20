# Design

`look_blocked(nav, physics, feet, dir, reach)`: with the level's `NavGrid`,
`line_walk(grid, feet, feet + dir·reach)`; blocked when the walk is blocked
and its clamped end lies within 1 m of the feet (horizontally). Without a
grid, the previous ray (`fixed_hit` at hip height, reach capped at 1 m).
The reach is the look-ahead uncapped, as the original's goal is.
Characters' footprints are not stamped into the live grid, so the player
does not block itself.

Measured: the scripted walk that got the player stuck against furniture
logged ~1 500 controller moves over 2 ms (mean 4.4 ms) and fell to 10 FPS;
with the grid look-ahead, 169 such moves, no stuck phase, and the player
still walks the room.
