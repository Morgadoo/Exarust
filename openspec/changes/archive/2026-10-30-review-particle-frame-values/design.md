# Design

- The per-frame values live on `LightState::frame` and `SoundState::frame`,
  called from `Emitter::frame` once a rendered frame after the flame's
  fixed ticks, so the walk draws happen at the original's rate.
- `Emitter::lod_scale` keeps the original's squared distance, summed
  `(x² + y²) + z²`; a NaN is kept as the unordered compare keeps it. It
  and `Emitter::camera_quad` are tested functions; no material reads them
  yet.
- The fire loop keeps its voices in a resource keyed by the flame entity,
  so a despawned flame's voice and buffer reference can still be freed;
  it replays only when its handle is unbound (the pool took the voice), so
  each flame holds at most one voice. A failed load is remembered and not
  retried each frame.
- `AudioEnvironment::play_buffer_attached` mirrors `play_loop_attached`
  for a named buffer.
