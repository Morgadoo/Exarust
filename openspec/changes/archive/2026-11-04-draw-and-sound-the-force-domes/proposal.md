# Proposal

## Why

Backlog item E5 (`context/wiring-backlog.md`): the Barrier's and the Bubble's domes (TForceBarrierEffect, TForceBubbleEffect) had their meshes, ticks, keep-outs and GI shapes ported, but nothing drew them, nothing ran their frame step (so the shown share the GI shapes read never blended) and none of their sounds played.

## What Changes

- `exa_render::force_effects`: the shipped `effectbarrier_v` with `effectbarrier_f` and `effectbubble_f` ported to WGSL (`force_dome.wesl`, `ForceDomeMaterial`, `BUBBLE` selecting the Bubble's fragment stage): `ViewPos`, `TexPos`, the eight `ColPt` points and `Fade`, `EmiTex` the `_ForceNoise` picture; the `_ForceBarrier01` record's render state (blend word 0x163, premultiplied; flags 0x480008, one-sided).
- `exa_game::effects::dome_draw`: each Barrier and Bubble runs its frame step every frame at the fixed step's fraction and draws its mesh (the cap or the sphere at the dome's radius, both windings) in the node's frame: the Barrier turned to its blended facing at its blended place, the Bubble unturned; the draw leaves with the dome.
- `exa_game::thaum::dome_sound`: the cast's `thmfrc_ss`, the looping `thmfrc_bc02` hum (volume 0, pitch 1, its start 0.8·r of its length in, then the frame's levels and place each frame, freed with the dome), the contacts' `thmfrc_bi`, and the Bubble's break `thmfrc_brf`.
- `effects::force_dome`: the Barrier's texture position follows its drawn place's height (not a clock); the dome's teardown names the hum voice it frees (it was called a material); docs name the cast sound and the bounding box.
- The domes count as force effects for the noise picture's ticks.
- The barrier logs why it ends (its hook called, with the events asked, or a focus shortfall).

## Capabilities

### Modified Capabilities

- `rendering`: the force dome programs.
- `thaumaturgy`: the domes' draw, frame step and sounds.

## Impact

- `exa_render::force_effects`, `force_dome.wesl`; `exa_game::effects::{dome_draw, force_dome}`, `exa_game::thaum::{dome_sound, barrier_live, bubble_live, effect_draw, live}`.
- Evidence: REA (Ghidra) pseudo-code of both geometries' program setups and uniform uploads, the dome's cast, mesh build, frame and destructor, the Barrier's frame, cast and keep-out, the Bubble's creator, cast, tick and keep-out; the `_ForceBarrier01` material routine and its 0xb8-byte record (emissive 0.25 ×3 and 1, opacity 0.5, blend word 0x163, flags 0x480008) read from `Exanima.exe`; the GLSL in `context/re/shaders/shading/`.

## Out of scope

- The per-supersample-layer passes and the level of detail's texture row offset (Exarust draws once a frame).
- The balance and knock count a contact drains; the strong self-pushes of a Bubble cast with its centre off the caster (the keep-out as ported).
- The Barrier node's turn is taken as the keep-out's cap frame (flattened facing, y up).
