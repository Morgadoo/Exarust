# Finish the shader builder's remaining entries

## Why

The shader builder and GPU program unit still lacked the builder's separate
shadow-declaration callback, the exact `SdwSmp` tap table, the material
loader's texture-upload wait, and had three arithmetic or state mismatches
against the original: the `Global` block's time lanes, the shadow-tap count's
integer wrap and the program cache free touching the drawing state's stale
flag.

This change advances **docs/PLAN.md Phase 5 (Visual and audio fidelity)**:
FLX_GPUPrograms and TShaderBuilder.

## What changes

- `shader_template::interface_shadow_samplers`: one `Shadow<i>` per light
  slot, each slot's kind tested on its own (`0x300 == 0x200` → 2-D).
- `shader_bindings::SHADOW_TAPS`: the eight three-float taps at their exact
  bit patterns; `shading.wesl` uses the first two lanes at full precision.
- `shader_uniforms::ViewInputs`: `subframe_offset` and `frame_scale` replace
  `time_delta`; `ViewPos.w = TimeDlt.x = offset · scale`,
  `TimeDlt.y = −(1/factor) · scale`.
- `shader_program::shadow_samples`: signed 32-bit exponents, shift counts
  modulo 32, signed divide.
- `shader_cache::ProgramCache::clear` keeps the stale flag.
- `texture_wait`: the first five occupied map slots, the slots started and
  the poll order.

## Evidence

The routines' assembly: the callback walks every slot with a 16-byte stride
and the same two declaration strings as the fragment interface (which tests
only slot 0); the tap table is 96 bytes of data read from the executable;
the `Global` writer multiplies the super-sampling loop's sub-frame offset by
the frame scale for both `ViewPos.w` and `TimeDlt.x`, and negates
`1/factor` then multiplies by the same frame scale for `TimeDlt.y`; the tap
count uses `shl` and a sign-extended `idiv`; the cache free only rebinds
program 0; the wait tests mask 1 of each texture's state word over at most
five slots.

## Out of scope

The GL compile/upload calls, the driver's `Sleep(0)` spin, the sub-frame
jitter matrix pair, and wiring: `GlobalUniforms::from_view`, the callback
and the wait have no caller in the running renderer (Bevy's pipeline and
asset server own those steps), so frame and load time are unchanged apart
from the WESL tap constants, which are the same eight values at full
precision.
