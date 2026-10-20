# Proposal

## Why

The Force, Energy and Displacement domains' casts, and the three effects
they lean on most (the electric arcs, the force barricade and the
displacement wisps), had no counterpart in Exarust beyond one stand-in
effect per domain. Their rules decide when a power may be cast and what it
costs, and their simulations decide what a power does to bodies.

## What Changes

- The shared cast protocol: the Force and Energy state gate, the two
  Displacement gates, persistent slots, the cast-motion start, the front
  ends' prices and the focus factors (single or double precision as the
  original computes them).
- The Force, Energy and Displacement tables, their casts, their AI
  evaluators and what each cast makes (the blast family's tuning, the
  barricade's placement, the domes' places, the throw pick, the shock's
  charge, the shock weapons' arming, the phasing streak and the phased
  weapons' restore, the wisps' anchors per bone, the AI's escape point).
- The electric arcs' simulation, level and limb tests, frame values,
  ribbon and light points.
- The force barricade's raising, trim, navigation stamp, tick, body shove,
  frame and mesh, and its power's end.
- The displacement wisps' setup, tick and strip.
- A plugin that runs live arcs, barricades and wisps against the bodies and
  the level, debug flags that put them on the player, an effect push on body
  parts in `exa_physics`, and an additive stand-in material in `exa_render`.

## Capabilities

### New Capabilities

### Modified Capabilities

- `thaumaturgy`: the cast protocol, the three domains' casts, the arcs, the
  barricade and the wisps.

## Impact

- `exa_game::thaum::{casting, engine, force, energy, displacement, arcs,
  barricade, wisps, live}`; `exa_physics::body::Ragdoll::push_sphere`,
  `Balance::set_counter`; `exa_render::glow`; `exarust` flags
  `--debug-arcs`, `--debug-barricade`, `--debug-wisps`.
- Docs: `docs/THAUMATURGY.md` (new), `docs/DEBUGGING.md`, `docs/INDEX.md`,
  the crate READMEs.
- Evidence: REA/Ghidra decompiles and assembly of the Force, Energy and
  Displacement unit routines, `TTendrilEmitter`, `TForceBarricadeEffect`,
  `TTendrilEffect` and the helpers they call; tables read from `.data`. No
  corpus file is read.
- Out of scope: the player's power bar and cast keys, the cast clips, the
  blast, torrent, dome, shell, kinesis and throw simulations, the effect
  shaders and the force noise texture, lights, sounds, the navigation grid's
  stamp application and the AI's sight of a barricade, the displacement
  render passes, the Mind domain.

## Phase

Phase 5, visual and audio fidelity: effects, thaumaturgy effects (force
waves, barriers, tendrils); the casts serve Phase 4's powers.
