# Model the shader program cache, bindings and light slots

## Why

The original's GPU program unit keeps a cache of compiled surface programs,
binds each program's uniform blocks and texture units, uploads the shadow
taps per sub-frame and fills six light slots per draw. Exarust had the
template expansion and program choice but none of these routines, so the
warm-up's real cache size, the light upload order and the material block's
dirt and displacement branches could not be compared with the original.

This change advances **docs/PLAN.md Phase 5 (Visual and audio fidelity)**:
the shader builder and the GPU program unit (FLX_GPUPrograms, TShaderBuilder).

## What changes

- `exa_render::shader_cache`: buckets keyed on the material and shader
  words (one 64-bit compare), entries keyed on the object and environment
  words, failed builds kept, reference counts, the bound program, the
  cache free and the warm-up pass; `exa-tool shader --warmup` prints the
  resulting cache.
- `exa_render::shader_bindings`: block bindings 0/1, the packed map units
  with the fixed environment, GI, screen-copy and shadow units, the six
  cached uniform locations and the `SdwSmp` window counted in `vec3` taps.
- `exa_render::light_slots`: the gather (view cap, at most six, weight
  below 0.004 ends, NaN passes), the unshadowed-first ordering that keeps
  last frame's slots, the per-slot position, colour, depth scale, filter
  radius and shaping, the stale flag and the reset.
- `exa_render::texture_program`: a `textures.flb` entry expanded as its own
  fragment program with the fixed `MPrj` matrix.
- `shader_program`: the draw's object word, the light-gather test and the
  warm-up step split; `shader_uniforms`: the material writer's dirt
  branches, the displacement gating and the writer's double-precision
  steps.

## Evidence

The routines' assembly in the shaders pack: the intern compare is a qword
compare of the material and shader words; the entry lookup a qword compare
of the object and environment words; the shadow-tap upload passes an
element count of taps (4, taps·4, 1 or taps); the light fill reads one
vector per light kind for the slot position, the copied softness for the
radius and a separate record field for the depth scale, caps the gather at
the view's limit, and the reset touches only the slot's light, kind and the
count. `MPrj` was read from the executable's data.

## Out of scope

The GL compile/link and draw calls, the geometry stage (no shipped set
uses one), the texture wait loop, the parent transform upload and the
texture programs' built-in vertex shaders. None of the new modules is wired
into the running renderer: Bevy builds one WESL pipeline with run-time flag
words and reads its clustered lights, so frame and load time are unchanged.
