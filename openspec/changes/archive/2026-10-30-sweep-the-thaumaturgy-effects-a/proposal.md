# Proposal

## Why

Phase 7 (thaumaturgy and effects) of docs/PLAN.md. A sweep over seventeen
small Thaumaturgy & effects components that were 50–92 % reproduced: the
flame row's draw, the portal particle system, the electric template tone,
the force domes' and barricade's GI lights, the power unit's spirit spawn
and deferred calls, the force wave's mesh and loop, the throw's sub-nodes,
the tendril smoke program, the strikes' light shape, the danger hazards,
the portal spray's life word and the fire unit's colour ramp. The lane's
work was reviewed against the assembly and several rules were corrected
(the Bubble's hum, the Barrier's and barricade's GI frames, the electric
peak opacity and brief draws, the spirit's ground height, the wave loop's
gain, the throw's voice flag, a duplicated flame-light descriptor).

Evidence (REA, Ghidra listing of `Exanima.exe`, and the objdump listing):
the flame row's draw and constructor, its `_FlameMat01` program text and
palette table in the executable's data, the portal unit's import handler,
burst start and spawn-one routine, the electric template selection, the
Barrier's and the Bubble's frames, the barrier, bubble and barricade
emissives' draws, the barricade's frame, the spirit form's body creation
and the deferred power call, the wave's constructor and cast, the throw's
grab, lift and end, the shipped `effectsmoke_f` and `gi_cuboid_fn`
programs from `shading.flb`, and the fire unit's object conversion.

## What Changes

- `exa_game::effects::fire_row`: the per-frame ribbon (16 sections of 8
  vertices) and its mesh; `exa_render::flame_row`: the `_FlameMat01`
  material and its 1 × 5 palette.
- `exa_game::effects::portal`: the import handler and the dissolve ring;
  `portal_effect`: each particle starts with the system's lifetime.
- `exa_render::electricity`: the template selection's tone, brief choice
  and peak tone.
- `exa_game::effects::force_dome`: the Bubble's and the Barrier's hum
  precision; `thaum::effect_light`: the Barrier's `gi_dome`, the Bubble's
  `gi_bubble` and the barricade's `gi_cuboid_fn` registered while they
  live, and the strikes' `gi_gradline` builder; `exa_render::gi_shape` and
  `gi.wesl`: the `gi_cuboid_fn` body.
- `thaum::power`: the spirit spawn and the deferred power calls;
  `power::force`: the throw's sub-node collection; `effects::force_wave`:
  the mesh plan and the charge loop; `effects::danger::Hazard` on the live
  blasts and waves; `exa_render::force_effects`: the `effectsmoke_f` body;
  `effects::fire_node::flame_ramp`.

## Out of scope

The per-supersample scissor loops (Exarust draws once), the flame row's
and fire node's live callers (no item carries the burning ability), the
portal particle draw and its seeding caller, an AI pass that reads the
hazards, the spirit character's generation and the scene's placement
query, the electric tone's caller, the strikes aura's line points, the
force domes' own `effectbarrier`/`effectbubble` programs and sounds.
