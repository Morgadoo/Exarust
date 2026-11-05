## Context

`--fixed-frames` makes each frame advance 1/60 s, so ticks per frame are
fixed once the game runs; what varied was when the body spawned relative to
the ticks and the physics world's collider order.

## Decisions

- Hold only the fixed step during the load (a run condition on Bevy's fixed
  main loop set), not the frames' clock: the loading stages that wait on
  fires and the GI warm-up run on frame time.
- Keep the builds on the pool; order their results instead of waiting for
  all of them, so the load is not slower.

## Open questions

- None open for determinism in the three scenes checked; `--sim-digest`
  over a whole story level is too slow to run in the grid (seconds a step).

## References

docs/UI.md (loading stages), docs/DEBUGGING.md.
