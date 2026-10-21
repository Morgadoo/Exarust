# Expand the original shader templates

## Why

The original draws every surface with GLSL generated at run time from the
templates in `shading.flb`: a builder expands conditionals, loops and
expressions for each combination of material, shader, object and
environment flags, and a material factory picks the program and the flags
from the material record. Exarust had hand-written WESL only, with no way to
see the program the original compiles for a material, and several WESL
terms that did not match the original's maths.

This change advances **docs/PLAN.md Phase 5 (Visual and audio fidelity)**,
the render-pipeline milestone: the shader builder and the material program
set.

## What changes

- Decode `.flb` shader libraries (container, per-entry cipher, NUL
  padding) in `exa_formats::flb`.
- Expand templates as the builder does (`exa_render::shader_template`): the
  word-group condition rules, the variables' truth tests, the `&` chain, the
  else rules, loops, the builder's number printing, includes and the
  generated vertex and fragment interfaces.
- Build the material and shader words as the material factory does and
  choose the program, the shadow and displacement sample splits and the
  warm-up keys (`exa_render::shader_program`); the `Global`, `Material` and
  light uniform values (`exa_render::shader_uniforms`).
- `exa-tool shader` prints a material's generated program, lists the
  warm-up keys, surveys a library's records (`--all`) and dumps the decoded
  entries (`--dump`).
- Correct the WESL surface program where it differed from the decoded
  templates (unlit emissive alpha, two-sided normal flip, PBR f0 without a
  diffuse map, skin alpha, displacement-shadow normal, GI tap frame, N·H
  clamp, specular GI without a reflection layer inside the GI volume,
  parallax ambient darkening in the PBR program only, legacy diffuse ×1.5
  before decoding, the specular ×3 rule, the parallax texel rounding).

## Evidence

- REA (Ghidra) on `Exanima.exe`: the shader builder (walker, command,
  condition, expression, interface and constructor routines, checked in the
  assembly for signed tests, the `=` scan, the chain's `!` and the rounding
  instructions), the float printer, the material loader and factory, the UBO
  writer, the light-slot fill and the warm-up table (compared byte for byte
  with `WARMUP`).
- Corpus: all 54 entries of the installed `shading.flb`, decoded by
  `exa-tool shader --dump`, match an independent decode byte for byte; every
  entry expands without leftover markers; the four shipped material
  libraries surveyed with `--all`.

## Out of scope

- Building or drawing the procedural texture programs of `textures.flb`
  (their variable tables), the geometry stage, the effect programs' own
  interfaces, a runtime program cache and GL compilation; Exarust keeps
  its WESL programs, which the generated text documents.
- The hair and translucent programs in WESL, `LocalRef` screen-space
  reflection, level fog on materials, wetness, the forward tone map and
  sharpen, dirt modes 1–3 and the `DispSdwF` pre-pass.
- A capture of the programs the running game compiles.
