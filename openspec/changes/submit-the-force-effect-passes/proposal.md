## Why

Rounds 20–23 ported the Force effects as CPU models (the five effect materials, the noise picture, the four layered passes, the blast and torrent ribbons, the force-shell aura and its uniforms), but nothing reached the GPU: the blast and torrent ribbons drew with the additive stand-in and a Shell cast's aura drew nothing (docs/PLAN.md Phase 6 notes: "GPU submission of the effect passes" remaining).

## What Changes

- `exa_render::force_effects`: the shipped `effectforce_f`, `effecttorrent_f` and `effectshell_v`/`effectshell_f` programs ported to WGSL as Bevy materials, with the materials' record render state read from `Exanima.exe` (blend word 0x163 = ONE, ONE_MINUS_SRC_ALPHA, the force record two-sided, the aura record one-sided), and the `_ForceNoise` texture (one red byte a texel, its mip chain).
- `exa_game::thaum::effect_draw`: the noise picture made from the engine's noise object on the first frame a force effect is alive and uploaded only when the object reports a new picture; the blast-family and torrent ribbons drawn with the ported programs, their frames' `TexOfs` and the second coordinate set; a Shell cast's aura drawn as copies of the caster's skinned meshes with the CPU model's uniforms each frame.
- naga composition tests for both programs and uniform layouts checked against their WGSL structs.

Phase 6 (thaumaturgy notes under Phase 6) in docs/PLAN.md.

## Evidence

The material routines' record blocks (`_ForceMat01`, `_ForceAura01`: blend word 0x163, flags 0x480018 / 0x480008) and the blast program setup (`EmiTex` on unit 0, `TexOfs` = (0, radius, scale)), read with REA from `Exanima.exe`; the GLSL in `context/re/shaders/shading/`.

## Out of scope

The barrier, bubble, barricade, strikes, tendril and displacement-shell programs; the per-supersample-layer pass loop (Exarust draws each effect once a frame); the force record's own texture slot (the noise picture stands in as `EmiTex` for the ribbons).
