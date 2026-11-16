## Why

The second oracle pass checked the engine's frame, matrix, negation and
fast-length routines but left their callers' ports on glam's forms, and it
found that the barricade's AI place score measured from the wrong point and
wrote the wrong word (F569). More floating-point-heavy rows with a few
helpers could be checked the same way.

## Evidence

The original's code run in the emulator on the build worker, 5000 fuzzed
cases per function, replayed against the Rust ports; for each switched call
site the caller's assembly shows the engine routine called there.

## What changes

- Call sites take the engine's checked routines where the original calls
  them: the door press's fast length, the wisps' fast distance, the sector
  point routine on the level detection's static-only path, the relative
  frame and point routine in the placement and overlap touches, the point
  routines in the box meet tests, the scaled push for the weapon grabs and
  the grip links, the `0 − v` negation where the original negates in place,
  and the rigid inverse of the emissive light shapes.
- The barricade's place score follows the original's avoidance slot (F569).
- The head look's angles are split out with the original's pitch offset.
- A light's derive and node colour linearise by the original's sRGB routine.
- `exa_oracle`: the barricade place, the head look and the light update.

## Out of scope

The render-only products (the sun's texture-plane dots and products, the
GPU material matrices), the foot pitch's relative frame, the barrier's
keep-out frame, DEC-7.
