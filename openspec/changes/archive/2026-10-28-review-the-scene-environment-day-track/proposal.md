# Proposal

## Why

docs/PLAN.md, Phase 2 (static world), the scene environment and grid
environment paragraphs: the outdoor arenas took a stand-in haze for the day
record's fog and the level's own colour for the sun, because the engine's
built-in time-of-day track had not been decoded. The sceneenv2 lane decoded
it; review found it linearised at start-up, which the shipped game never
does.

## What Changes

- `exa_world::scene_env`: the built-in six-record day track
  (`EnvironmentTrack::day`, keys 0.38 to 0.70, every value checked against
  the image) and its start-up pass: the `o` fill rule always, the
  linearise-and-scale-by-0.75 step only under a flag the shipped game never
  sets (`TRACK_LINEARISED_AT_STARTUP = false`), and the frame's horizon
  linearised because that flag is clear. Every scene starts with the track.
- The frame publishes the sun-height scene value (−100000 / −25), the
  rain-driven wind (rain × 0.3) and the tint (colour × its fourth slot).
- `_Sunlight` node constants (kind 2, 3 m range) and `Sun::range`.
- `environment`: under a sky the sun light takes the record's sun colour as
  stored and the fog the record's fog; the haze stand-in is gone.
- `grid`: the wall and floor set tables by index and `sector_lists`;
  `rooms::cell_bytes` (the character's unclamped cell bytes); the rain
  effect gate.
- Review: the lightning flicker's NaN step turns up again (the lane sent it
  down), the duplicate `tint_strength` field is gone.

## Evidence

The track's 330 floats compared one by one with the image; the start-up
routine's two passes, the linearise and 3-wide scale helpers, the lerp, the
blend's post-linearisation gate, the frame apply's horizon gate, wind,
sun-height compare and tint copy, and the tick's flicker compare read from
the assembly.

## Out of scope

A frame system driving the weather tick and time of day; the sky shader,
the environment cube's masked tint upload, the sky object's node writes,
the rain and thunder sounds; the grid's per-cell floor/ceiling mesh merge
and the cap's own strip tables.
