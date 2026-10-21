# Proposal

## Why

Exarust's fires used a reduced model of the original's particle emitter
(`TParticleSystem`) with only the small template, and its electric arcs
had colours and part filters but no presets, record or renderer. A first
port of the full emitter, the six fire templates with their three beds and
the arc rules was made on a lane; a review against the original found that
it gave every torch particle an upward launch (the spawn used the velocity
retention where the original uses the template speed), faced the flames
from a camera position in metres, baked the colour tables with truncation
and dropped the beds' leading keys, and mixed up several arc fields.

## What Changes

- The particle emitter: template scaling, derived per-tick values, the
  ring of Verlet particles, fire ageing with its distance cap, the
  two-thirds accumulator, spawning from one source or a ring of them, the
  light walk and density, the sound walks, bounds, and the three quad
  builders with the atlas variants and colour keys. Its arithmetic follows
  the original's instructions (double-precision steps, half-to-even
  rounding, the fast arc cosine, the length's summation order) and its
  draws use the original's generators (the engine's integer LCG and the
  run-time library's Mersenne twister).
- The fire presets: the four tiers, their beds and lights, the load-time
  scaled key tables, one-shot large fires that are removed once finished.
- The carried torch's fire rides the hand as an emitter on the item's body.
- Hidden flames build no quads; a slow frame catches up four ticks at most.
- Electric arcs: the weapon, spark-burst and device presets, the 84-byte
  record, the charge an electrified item spends, the release and rebuild
  checks, the `_ElecMat01` shader and a jittered ribbon.

## Capabilities

### New Capabilities

### Modified Capabilities

- `rendering`: fires, flame particles, the released electrified effect;
  new requirements for the emitter tick, the fire keys, carried fires and
  the arc presets and record.

## Impact

- `exa_world::particles` (new), `exa_world::fire`, `exa_world::flame`,
  `exa_world::arc` (new); `exa_render::electricity`, `exa_render::arc.wesl`
  (new); `exa_game::torch` (marks its fire carried).
- Docs: `docs/RENDERING.md` (Fires, Electricity), the `exa_world` and
  `exa_render` READMEs.
- Evidence: reverse engineering of `Exanima.exe` (REA, Ghidra pseudo-code
  and assembly): the fire presets unit (template scaling, the tier builders,
  `CreateFire`, the key scaling and the marker import), the particle system
  (constructor, ring builder, descriptor copy, sim insertion, tick, ageing,
  integration, spawn, bounds, per-frame build, the three quad builders, UV
  tables and colour lerps), the electricity unit routines (release, charge,
  rebuild check, equipment check, presets, record, shader source), and the
  data: the seven template records, the six key tables, the engine
  constants, the vector helpers and the run-time library's random. No
  corpus fields change.
- Out of scope: the tendril emitter's tick (`TTendrilEmitter`, another
  component), arc callers, fires on other physics bodies, body contacts
  driven from the physics step, the fire's emissive scale on its parent
  mesh, the detail value per node, the quality-scaled pool override, and
  the looping fire and arc sounds.

## Phase

Phase 5, visual and audio fidelity (Effects: particles: fire, sparks).
