# Proposal

## Why

In an outdoor storm, each lightning strike cleared the frame to black and
pulled the fog in to a tenth of its span, instead of brightening the scene
as the original does. Exarust raised the scene's flash-clear switch whenever
a strike started, standing in for a flag whose writer had not been found.
With the constructor's zero flash tint, that branch clears with black.

Evidence (REA, Ghidra listing of the 0.9.5 build, and the objdump listing):
the frame apply reads the flash-clear byte in one place and no instruction
in the executable writes it, so it is always off in the shipped game. The
strike routine sets only the strike's own state (active, end time,
attenuation, thunder time, flicker step). While a strike runs, the frame
adds the lightning blue (0.2, 0.3, 1.0) to the record's fog colour by
I / (I + 2) · 3. The shader upload reads the ambient, the ambient scale, the
wind, the scratch colour and the fog span, none of which a strike changes.
There is no exposure change in the original's renderer to follow.

Phase: Phase 5 (world rendering), the scene environment's weather.

## What Changes

- `exa_world::scene_env`: a strike no longer turns on the flash-clear
  switch, and the strike's end and `clear_weather` no longer turn it off.
  The switch stays modelled, with its tint, for a caller that sets it.
  `Frame` publishes the running strike's intensity (`lightning`).
- `exa_world::environment`: `EXA_LIGHTNING_FRAME=<n>` (a debug aid) forces a
  strike 316 cm from the view from frame `n` on whenever none is running
  and logs the strike's frames.
- Tests on the original's numbers: the strike's attenuation (0.8775 at
  316 cm), the fog colour along the flicker, the clear, sun, ambient and fog
  reach kept, and the record's colour back at the strike's end.

## Out of scope

- Where the original's renderer draws the lightning-tinted record fog
  colour (the shader's fog uniform is the scratch colour; the record colour's
  reader is outside the scene environment). Exarust keeps drawing it as the
  outdoor fog colour, as before.
- The thunder sound, the rain effect and the bolt itself.

## Impact

`exa_world`; docs/RENDERING.md.
