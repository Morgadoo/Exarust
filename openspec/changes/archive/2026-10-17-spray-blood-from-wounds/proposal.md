## Why

In the original every bleeding blow throws a short jet of blood droplets
from the wound: a spray of red spatter that flies off the struck limb,
falls and fades within a second or two. Exarust's blows bleed on their
victims (`bleed-from-wounds`) and stain the blade (`stain-weapons-with-blood`)
but nothing flies.

In the original the damage routine's blood branch adds one jet to a
scene-wide droplet system for each bleeding blow: at the contact point,
fixed to the struck part, against the striker's swing, with a strength of
120·R (200 above R 1.67) that sets how many droplets it sends. The droplet
template (40 a second, 0.75 s life, 24 cm half-size, 48 cm/s, a quarter of
gravity) and the material (`_BloodPrt02`, the spatter atlas `BldPrt02`
multiplied into the frame) are built into the executable.

Phase 5 (Visual and audio fidelity), milestone M5 "looks and sounds like
Exanima", item "blood sprays". It also serves Phase 4's M4 "first duel".

## What Changes

- **Jet at the blow** (`exa_combat::wounds`): `Spray`, a message written
  by the blow reactions inside the bleeding branch (also when the wound is
  dropped from a full queue): the struck body and part, the contact point,
  the striking point's velocity and the droplet count; `spray_strength`
  (120·R, 200 once R exceeds 1.67 in double precision) and `jet_droplets`
  (round(0.3 × strength), halves to even).
- **Droplets** (`exa_character::spray`, new): `BloodSprays`, the scene's
  one spray system (a resource): jets fixed to the struck part's frame,
  one shared emission clock (⅔ gate, (83/40)·⅔ ticks per emission), the
  droplet spawn (jet frame × random velocity × 48/83, 0.15 of the jet's
  motion, averaged with the jet's previous droplet, seeded with twice the
  jet's axis in the part's frame; 0.8 ahead, 1.2 behind; 62 ticks), the
  Verlet step (a quarter of gravity, 0.95 a second kept), 513 ring slots
  with sizes of 12–24 cm, at most 512 live. Ticked in the fixed step after
  the body solver on the parts' tick poses; drawn each frame as
  camera-facing diamonds in one world-space mesh, between the last two
  ticks (alpha keys, the 32 atlas looks).
- **Material** (`exa_render::spray`, `spray.wesl`, new): `SprayMaterial`,
  the texel mixed toward white by the droplet's alpha, multiply blend,
  unlit, both faces, no depth writes, no shadows; `BldPrt02` loaded with
  the first jet (sRGB, clamped on both axes, the colour textures' LOD
  bias).
- **Debug helpers**: `--debug-spray bone,R[,dx,dy,dz]` fires test jets from
  the player's (or the character view's) body every second;
  `--pause-after-jet n` pauses the simulation n ticks after the first jet
  (`exa_game::sim_control`, a small flag-gated addition to lane D's debug
  file), so a jet can be shot in a fight. `exa_character::blood` shares its
  part-front search (`PartFronts`) and bone names (`bone_by_name`) with
  them.
- Docs: `docs/RENDERING.md` (Blood sprays), `docs/formats/RFI.md` (the
  droplet atlas's flags), `docs/DEBUGGING.md`,
  `docs/PLAN.md` (Phase 5), the READMEs of `exa_combat`, `exa_character`,
  `exa_render`, `exarust`.

Evidence:
- Reverse engineering of `Exanima.exe` 0.9.5 (REA, Ghidra, `exe.asm`): the
  damage routine's blood branch (its two jet calls: strength 200 above R
  1.67, else 120·R in single precision; the direction is the striker's
  smoothed tip velocity negated), the jet routine (life = round(0.3 ×
  strength) in double precision, the frame from the direction and +Z, the
  link to the struck body's matrix, the seed 2 × the axis in that frame),
  the spray template and its derived values, the particle tick (no fire
  ageing; jets lose one life per droplet sent), spawn (velocity, seed
  blend, 0.8/1.2 placement), quad (mode 4 with the camera's forward axis,
  per-slot sizes), colour (no parity) and UV routines, and the material
  payload and its blend mode 4 branch in `shadeopt_f`
  (`context/re/wounds-and-blood.md` §2.1 and §11, refreshed this change).
- Corpus: `BldPrt02`'s header (256², BC1, flags `0x20A80000`) and its four
  spatter cells, pinned by a test.
- Exarust in game through `gpu_run.sh` (light runs): a duel paused just
  after a bleeding blow, `--debug-spray` in the arena and the character
  view.

Out of scope:
- floor blood (the original has none from combat), the attacker's own
  blood when it strikes with a body part (creatures' slot), blood across
  saves, session end and respawn (Exarust has none);
- the original's distance fog on the droplets (Exarust has no distance
  fog yet) and its 8-bit frame clamping a lit value before the multiply;
- a retail capture of a jet (retail request RR35, not taken).

## Capabilities

### New Capabilities
<!-- none -->

### Modified Capabilities
- `blood`: bleeding blows spray jets of droplets from the wound.
- `world-viewer`: the `--debug-spray` flag and the `--pause-after-jet`
  scripted pause.

## Impact

- `exa_combat` (`wounds.rs`: `Spray`, `spray_strength`, `jet_droplets`,
  the constants; `lib.rs`: the blow reactions send the jet)
- `exa_render` (new `spray.rs`, `spray.wesl`; `SprayPlugin` in the render
  plugin)
- `exa_character` (new `spray.rs`; `blood.rs`: `PartFronts`,
  `bone_by_name`; `lib.rs`: the plugin; `exa_physics` becomes a
  dependency for the bodies' tick poses)
- `exa_game` (`sim_control.rs`: `pause_after_jet`)
- `exarust` (`main.rs`: `--debug-spray`, `--pause-after-jet`)
- `docs/RENDERING.md`, `docs/formats/RFI.md`, `docs/DEBUGGING.md`,
  `docs/PLAN.md`, the READMEs of `exa_combat`, `exa_character`,
  `exa_render`, `exarust`
