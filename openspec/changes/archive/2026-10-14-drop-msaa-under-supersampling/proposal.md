## Why

The supersampled scene was also drawn with Bevy's default 4× MSAA, which
gave 16 samples per window pixel. The original multisamples nothing
(`HWMultisample = 0`): its four sub-frames are its only antialiasing. The
extra samples cost about a fifth of the frame and add nothing the original
shows. Measuring the frame rate also needed a way to present frames
without waiting for vsync.

## What Changes

- The supersampled scene camera draws without MSAA.
- `--no-vsync` presents frames as soon as they are drawn, for measuring.
- The frame-rate readout also logs each reading at debug level.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `rendering`: antialiasing.

## Impact

- `exa_render` (`supersample.rs`), `exa_ui` (`fps.rs`), `exarust`
  (`--no-vsync`), `docs/RENDERING.md`.
