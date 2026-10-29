## Why

The material factory picks the hair program (`shadehar_f`) for every type-3 record, but Exarust drew hair with the PBR program, so the strands showed the textured PBR look instead of the original's flat picked colour with an anisotropic band and a wrapping rim. Every program also drops its specular sum for a record without `MatSpc`, which Exarust ignored.

## What Changes

- A `HAIR` shading flag from the program choice; `exa.wesl` branches into the hair program (`hair.wesl`), mirrored and pinned in `exa_render::hair_program`.
- A `NO_SPECULAR` shading flag for a lit record whose material word lacks `0x4`; every program leaves its specular sum out.

Phase 5 (lighting fidelity) in docs/PLAN.md.

## Evidence

The decoded `shadehar_f`, `shadeopt_f` and `shading_f` templates (`exa-tool shader --dump`) and the shipped hair record `harhps1` (type 3, material flags `0x720007`, extra `0x100`, emissive alpha 1).

## Out of scope

The translucent program, wetness, and the outdoor branch's GI colour in the hair reflection.
