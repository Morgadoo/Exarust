# Design

- `Emitter::quad_axis` takes the camera axis through `rotation.transpose()`
  (the original's matrix-times-vector sums `(x·a + y·b) + z·c`, which glam's
  column sum reproduces) and the view-plane flatten through
  `exa_physics::engine_math::normalize_approx`; the axes are worked out
  once per frame before the quad loop, as the original does.
- The diamond's facing line and the stretched quad's cross products use the
  same approximate normalisation; a zero line gives a zero axis, as the
  masked reciprocal does.
- `load_scale_legacy` reuses `exa_assets::human_skin::srgb_encode` (now
  public, NaN down the power branch) and `exa_formats::colour::{desaturate,
  channel_byte}` instead of a local copy.
- `Arc::pass_clock` keeps the counter's integer add and single-precision
  0.5 and works the rest in double, like the force geometries' pass row.
- `ArcSpec::records` separates the allocated tendril records from the
  ribbon model's drawn tendrils.
- `drive_hosts` finds a host's fire through its children.
