# Tasks

- [x] Verify the lane's per-frame light, sound, deadline, detail, ageing,
  bounds, camera quad, level-of-detail and arc-scroll rules against the
  assembly.
- [x] Fix the level of detail (squared distance), the camera quad's axis,
  the arc scroll's wrap and double-precision step, the spawn pull's axis,
  the sound bounds' NaN order and precision, and the bounds' NaN radius.
- [x] Bind each flame's loop to one handle, free it with the flame, and
  stop retrying a failed load; add `play_buffer_attached` with a test.
- [x] Update `docs/RENDERING.md` and the `exa_audio` README.
