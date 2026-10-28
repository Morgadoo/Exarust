# Model the per-draw state and correct the uniform values

## Why

The GPU program unit's per-draw function decides, around Bevy-owned GL
work, which lights a draw gathers, its object word, the material block's
fade and emissive rewrites and the super-sampling sub-frames with their
tap windows and clocks. Exarust had none of this, and several `Global`,
`Material` and light values disagreed with the executable (the time pair,
`FogPos`, the refraction background, the factory's emissive and specular
branches, the light colour and filter precision).

This change advances **docs/PLAN.md Phase 5 (Visual and audio fidelity)**:
FLX_GPUPrograms (unit routines) and TShaderBuilder.

## What changes

- `exa_render::draw_state`: the per-draw plan (light gather or clear,
  object word, fade rewrite past the engine epsilon, per-draw emissive
  scale, liquid level and GI corner uploads, sub-frames with `SdwSmp`
  windows, animated clocks and motion-blur shutter times), the single-pass
  variant, the scene begin and the effect parent matrix.
- `exa_render::shader_uniforms`: `ViewPos.w`/`TimeDlt` from the frame clock
  and the sample factor; `FogPos` as the eye/sun-eye midpoint with `w` the
  scene's height limit capping `y`; the unlit diffuse, masked-word emissive
  doubling and specular choice; the emissive exposure gate; the refraction
  background chosen by the scene's switch, a double compare with 0.85 and
  local reflections; light colour and radii in the engine's double steps.
- `exa_render::shader_template`, `texture_program`, `shader_program`: the
  geometry stage and effect pairs, the six built-in texture vertex
  programs (byte-identical to the executable's text) with their stream
  selection, GI mode 3 for `VXGISpec`, the half-pass rule on the last pass
  index.
- `exa_assets::model`: the height layer switches off only for a
  one-texel-tall map; the parallax, step and bias are scaled in double.

## Evidence

The executable's assembly for the per-draw function, the single-pass
variant, the light-array writer, the material writer, the material factory
and the `Global` builder; constants read from the executable (`2/π`,
`0.005`, `0.003`, `0.85`, `2048`, `3e-5`, the epsilon `0x33FFFFD7`, the
scene height default 340); the built-in vertex texts compared byte for byte.

## Out of scope

The GL calls themselves (program creation and linking, buffer objects,
texture binds, matrix products and draws), the `SdwSmp` offset table, a
run-time `.flb` program cache in the live renderer, and drawing refraction
backgrounds (Exarust has no liquid pass). Only the height-map rule is wired
into the running game; the rest are tested models.
