## Why

The original's frames carry more texture detail without shimmering. At the
player's `Supersample = 4` it draws each frame four times with sub-pixel
jitter, averages them into the window-sized image, then post-processes at
window resolution. Exarust drew at window resolution only.

## What Changes

- `exa_render::supersample`: a `Supersample` scene camera renders at twice
  the window's resolution on each axis into an image. A display camera
  resolves it to the window by the equal-weight 2 × 2 average and runs the
  post chain.
- The display camera is ignored by the GI pass, flames, fog and the
  camera-following GI.
- The scene image follows window resizes.
- Viewer: on by default; `--no-supersample` turns it off.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `rendering`: supersampling.

## Impact

- `exa_render` (`supersample.rs`, `resolve.wesl`, `post.rs`, `gi.rs`),
  `exa_world` (camera queries), `exarust` (viewer) and `docs/RENDERING.md`.
