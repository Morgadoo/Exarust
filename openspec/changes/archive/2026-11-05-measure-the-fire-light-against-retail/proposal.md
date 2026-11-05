# Proposal

## Why

Round 57's GI lane left two lighting leads. First, the floor torch logged a
fire light intensity of 4.49 against a template intensity of 3.58 (× 1.05
flicker at most), so the fire density looked like it ran above 1 and might
add to the warm o3/o4 arena nights. Second, a fit of the menu scene against
one retail capture read Exarust's direct light at about 0.8 × retail's in
every channel.

Both were checked against the running original (its menu scene, read through
the process memory with the menu camera) and its code:

- The fire tick computes the density as `0.8·d + 0.2·live/25` from the slots
  with life above zero (the normaliser is the template's 450 scaled by
  `0.9s² + 0.1s`, rounded half to even), unclamped, and the frame's intensity
  as `density · (1 + 0.05·walk) · base`. The original's two menu torches
  average a density of 1.149 to 1.188 (0.72 to 1.49) over 3000 frames, with
  28.7 to 29.7 live particles, and an intensity of 4.10 to 4.19 (2.6 to 5.6).
  Exarust's torch averages 1.16 with 29.0 live. The density above 1 is the
  original's, not an excess: nothing to fix.
- The menu's lights in the original's staged light uniforms: colour
  `linear(colour) · intensity · 2/π`, inverse range `0.8 / radius`, shaping
  (1, 1, 1) for these unspotted lights, shadow radius `(softness + 1) · 0.005`;
  the fire lights 5 cm over their markers, the omni light at its record's
  place. Exarust's lights match all of these (positions to 0.1 mm). With
  Exarust's fire lights held at the original's mean (1.159 × the template)
  and 30 retail frames averaged, the fit reads direct × (1.10, 1.05, 1.01)
  and GI × (1.03, 1.08, 1.14), median errors 2–3 %, and the matched patches
  agree within 1–4 %. The 0.8 was one retail frame caught with the fires
  above their mean (single frames give direct factors 1.07 to 1.36).

## What Changes

- No change to the fire or light model.
- `EXA_FIRE_LIGHT=<f>`: a debug tool that holds every fire light at `f` ×
  its template intensity, for comparing a still frame with an average of the
  original's frames.
- `--debug-lights` in a level view (no player) lists the lights around the
  camera.
- A test pins the torch's average density and live count to the original's
  measurement.
- `docs/RENDERING.md` (Fires: the density's range), `docs/DEBUGGING.md`, the
  `exa_world` and `exa_game` READMEs.

## Capabilities

### New Capabilities

### Modified Capabilities

- `rendering`: Fires (the light's average above the template intensity).
- `world-viewer`: Light comparison options (the camera in a level view).
  Added: Fire light pin.

## Impact

- `exa_world::flame`, `exa_game::light_probe`. Default rendering unchanged.
- Evidence: `context/artifacts/2026-10-30-firelight/`.

## Out of scope

The remaining few per cent between the menu frames (the desk's shadow has
retail's blue at 1.5 ×, likely the omni light's GI share). The original's
flame particles were not compared shot by shot.

## Phase

Phase 2, static world: lighting.
