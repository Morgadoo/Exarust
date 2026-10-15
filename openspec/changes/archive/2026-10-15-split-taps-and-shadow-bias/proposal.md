## Why

Under supersampling each sub-frame took all eight shadow taps and four GI
taps. The original spreads its eight taps over its four sub-frames, two
each, so every output pixel still averages eight. Taking all of them is
four times the original's filter work, and it hid a bug: once each pixel
averaged only the original's eight taps, the lit terrain floor showed a
regular pattern of shadow acne that darkened it. The original avoids that
acne with per-node shadow-bias levels: terrain and floors keep about 1.5 %
of the light distance clear of their own shadow, not 0.5 %.

Two problems in unattended screenshots also spoiled comparisons. The
viewer window takes the focus when it opens, so typing elsewhere steered
the camera. The scene image also kept its first size when the window
settled late.

## What Changes

- In a supersampled frame each 2 × 2 block of the scene image is one
  window pixel. Its four pixels take taps 2i and 2i + 1 of the shadow disc,
  turned per window pixel, as the original's sub-frames do.
- The GI lookup takes the same disc taps as the original: half a unit
  across the normal, 20 cm out, split the same way.
- A shadow-bias level per mesh, carried in the mesh tag: 3 for terrain
  and tile-grid floors, 1 for other grid set pieces. It sets the receiver's
  self margin 0.995·k_c/k_r.
- The supersampled scene image follows the window's size every frame.
- The viewer's free camera ignores input while an unattended screenshot
  is pending.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `rendering`: shadow and GI taps, shadow-bias levels.

## Impact

- `exa_render` (`shading.wesl`, `exa.wesl`, `supersample.rs`,
  `material.rs`, `lib.rs`), `exa_world` (`terrain`, `grid.rs`), `exarust`
  (`viewer.rs`) and `docs/RENDERING.md`.
