## Why

A debug aid, not the original's (lane `workergpu`). The runs on the build
worker's GPU (`--offscreen`, a few hundred frames a second) lost about one
`--screenshot` in five: the run ended a fixed 20 frames after the shot was
asked for, before the render world had read it back, and Bevy logged
"Failed to send screenshot: sending on a closed channel" with no file
written.

## What changes

- A `--screenshot` run ends once its shot is captured and saved (an
  observer on the screenshot beside the save), not 20 frames later.
- If no shot arrives within 600 frames the run warns and ends anyway.

## Impact

- `crates/exarust/src/viewer.rs` (`screenshot_after_frames`).
- A run's last trace rows: a `--fixed-frames --trace` run with
  `--screenshot` now ends a few frames after the shot instead of 20.
