## Why

The original shows its frame rate in the top-right corner ("57.25 FPS"),
drawn in its own bitmap font. Exarust had no interface at all and no way to
read its frame rate.

## What Changes

- Read the game's base font atlas (`fontbase24r`) at run time: a Latin-1
  grid of upside-down glyphs, with proportional advances measured from the
  ink.
- Show the mean frame rate, refreshed every half second, top right, in that
  font at the original's size in physical pixels.
- `--no-fps` hides it.

## Capabilities

### New Capabilities
- `user-interface`: the game's bitmap font and the frame-rate readout.

### Modified Capabilities
(none)

## Impact

- `exa_ui` (`font`, `fps`), `exarust` (`--no-fps`), `docs/UI.md`.
