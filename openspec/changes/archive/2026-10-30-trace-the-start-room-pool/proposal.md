# Proposal

## Why

A side-by-side of the retail exanima01 start room against Exarust reported three differences: a cold white pool of light about 3 m wide around the player in retail and none in Exarust; distant walls about three times darker in Exarust's GI (2.7 against 8.8 sRGB on the top wall); and a redder Exarust frame (R 34.5 against 24.2). Tracing them showed:

- The pool is not the player light, the awareness value or a view-dependent specular. The player light stays off at a normal start (the equipment switch only turns it on for night-vision headgear, and the retail save wears none). The pool comes from two level lights over the spawn: a grey light 4 m up, intensity 60, radius 630 cm, GI multiplier 0.1, shaped straight down (q 0, p 15); and a faint white fill 2.5 m up, intensity 0.1, radius 2450 cm, GI multiplier 0. Exarust already draws both. With no torch in hand, Exarust's pool matches retail's (pool mean 66.9 against 70.6 sRGB). The torch Exarust's exanima01 start puts in the player's hand drowns the pool. Retail's story start has that torch lying on the floor beside the player.
- The darker walls were mostly a framing error. The comparison squashed a 1920×1080 render at FOV 25.1 to 1600×1080. Rendering at the retail window size with the retail FOV (1600×1080, FOV 29.93) lines the frames up. Retail's own distant surfaces also vary from frame to frame: the top wall reads 9.0, 6.3 and 2.9 in three captures 2 s apart. At the matched framing, with the torch laid on the floor, the whole frame measures (25.5, 16.9, 11.4) against retail's (24.2, 17.0, 12.2), and the top wall 5.8 against 2.9–9.0.
- The redness comes from the torch held at shoulder height, not from the GI ambient, the torch colour or the tonemapping.

While the light activation was being read, its range rule turned out to need a stored radius above zero as well as the flag. Exarust tested the flag alone.

## What Changes

- A light falls off with distance only when its node has the range flag (`0x40000`) and its stored radius is above zero (a NaN radius is not). Otherwise it reaches everywhere at full strength. No shipped light has the flag without a radius (1439 lights checked).
- New debug options, all in Exarust only: `--debug-lights M` logs the point lights within M metres of the player once, two seconds into play, with the GI base ambient and the scene ambient; `--no-torch` starts play without the carried torch; `--floor-torch x,y,z` lays a lit torch on its side at a point, as the story start has it.
- Tests with the original's numbers for the start room's two lights (shaping (0, 16), the pool halving about 1.5 m out on a floor 3.9 m down, ranges 7.875 m and 30.625 m), plus a check of the installed exanima01.

## Capabilities

### Modified Capabilities

- `rendering`: the light range rule.
- `world-viewer`: the light comparison options.

## Impact

- `exa_formats::rfc` (`ranged`), `exa_world` (tests), `exa_game::light_probe` (new), `exa_game::torch` (`FloorTorch`), `exarust` (options); `docs/RENDERING.md`, `docs/PLAYER.md`, `docs/DEBUGGING.md`, `docs/formats/RFC.md`.
- Evidence: the light activation and field derivation in the executable (range flag and positive radius give 0.8/R and the radius as the far plane; otherwise 0 and the unbounded far plane); the player light's equipment switch; the retail save's character database (the background-0 start letter, no night-vision helm, no torch in hand); the installed exanima01's light records; matched-framing captures in `context/artifacts/2026-10-30-lightpool/`.
- Out of scope: the story start's torch on the floor instead of in the hand (a game-flow change for its own lane). Also out of scope: the dim periphery, where retail's neutral light is about 1.7 times Exarust's at matched framing. Exarust's periphery is lit only by the fill light's direct term, and retail's extra is not explained yet. Settling it needs a capture of the retail GI volume.

## Phase

Phase 2, rendering fidelity (docs/PLAN.md): level lights and the retail comparison of the first story level.
