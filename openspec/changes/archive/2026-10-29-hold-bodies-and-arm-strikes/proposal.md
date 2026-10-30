## Why

The Force power classes ran Kinesis, Throw and Strikes without what they put on the world: a held body kept its gravity and damping, no shell showed on it, the pick made no sound, and Force Strikes armed no weapon (docs/PLAN.md Phase 7, thaumaturgy notes: "the kinesis's held material, pick shell and pick sound, and the throw's shell and material, are not wired"; "the aura builder" missing from Strikes).

## What Changes

- `exa_physics`: the held environment (no gravity; the per-second retentions 0.125, 0.0125 and 0.025 by speed band) and a body's hold in it (`Ragdoll::hold_in`, `release_held`), replacing the body's gravity and retention while set.
- `exa_game::thaum::hold_live`: a Kinesis pick and a Throw from the cursor put the body in the held environment under a force shell (colour 2.8, fade-plane scale 0.5, texture scale from the body's bounding radius); a throw from a kinesis takes both over; the kinesis's end, the throw's drop, restore and end close the shell and give the environment back. The pick plays `thmfrc_knsst`.
- Force Strikes puts an aura on each item its caster holds (at most four), removes it when the item is put away, steps its ripples while running and closes every aura when it ends.
- `exa_render::force_effects`: the strikes program `effectfrcstrk_f` as a `STRIKES` variant of the shell program with its `Gradient` uniform; rigid copies of a held body's or weapon's meshes drawn in the body's first part's frame.
- `--kinesis-pick <frame>`: a scripted pick for unattended runs.

## Evidence

REA/Ghidra on `Exanima.exe`: the kinesis pick callback, start/end and node walk; the throw's creators (cursor and handed), physics setup, tick, sound and end; the shared held-body shell creator; the shell constructor's defaults; the strikes activation, aura builder, weapon callbacks, tick and end; the strikes geometry's program setup and per-frame uniforms; the held environment's eight-word record and its conversion. GLSL `effectfrcstrk_f` from `context/re/shaders/shading/`.

## Out of scope

The held body's shell hum, the swing ripple on a strikes aura, the GI box lights of the held body and the weapon, the kinesis's 786-byte navigation sweep, the simulation flag set on held nodes (its reader was not found), a multi-node weapon's gradient radius.
