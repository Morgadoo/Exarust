## Why

Finding F550 (workergpu lane): outdoor arena 2 (`play arenao2 --env-seed
777`) showed thin light lines in every direction over its shadowed ground,
on main and on the terrain lane's branch alike. They were not terrain
triangle edges: `EXA_RAIN_DRAW=0` removes them and `EXA_RAIN_DEBUG=1`
draws them magenta. The rain's drawn vertex count is
`round(4096·intensity)` and is odd for about half the intensities. Exarust
draws its one-window-pixel-wide streaks as `SCALE` copies of the drawn
vertices in one line list, so an odd count paired every later copy's tails
with the next drops' heads: lines across the 6 m rain box.

## Evidence

REA (Exanima.exe), the rain effect's per-frame routine read in the
pseudo-code and its draw calls: per sub-frame it sets the corner's viewport
and matrices and calls the line draw (`GL_LINES`, first 0, the stored
count) on the same buffer, and on the reflected set again when the
reflection level is above three. An odd count leaves the last head without
a tail, and GL ignores that vertex; every corner draws only whole lines.

## What Changes

- `exa_game::weather::streak_vertices` takes the drawn vertices down to an
  even count before it repeats them for each supersample copy, so each
  copy draws whole lines, as the original's line draw does.
- Test: `an_odd_drawn_count_keeps_each_copys_lines_whole`.
- `docs/RENDERING.md` (Rain) and the game crate's README note it.

## Impact

Rain streaks only. The second F550 symptom, the straight seam in the
shadowed ground at world x = 0, is the voxel GI's per-object block pick at
a 48 m block boundary (it goes with `--no-gi`), not the terrain or its
shadows; it is left to the GI manager's lane.
