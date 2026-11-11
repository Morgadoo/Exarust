# Proposal

## Why

TShellEffect (rated 70%) still lacked the Constant Displacement's shell (its
creator lives in the constant's start, outside the class, and nothing in
Exarust made it), a vanish's reuse of that shell, the fade half's blended
body and the hidden body's shadow. A fidelity bug also drew every fire with
the large `FirePrt01` texture: the presets name the materials `_FirePrt02`
and `_FirePrt01`, and the flame draw compared against `FirePrt02`.

## What Changes

- `exa_world::flame`: a fire's texture name selects the small fire's
  material with or without the underscore. The original's `CreateFire`
  builds the small fires (up to 0.35, torches included) with `_FirePrt02`
  and the medium-small, medium and large flames and their beds with
  `_FirePrt01`; torches now draw `FirePrt02`.
- `exa_game::thaum::vanish_live::ConstantShell`: the Constant Displacement's
  start makes the body's `_EffectShell01` shell before the tendril image (its
  two creation draws first), leave-to-finish cleared; it opens over 400 ms,
  rests open while the power runs and closes when it ends, drawn over the
  caster's skinned copies with `Interior` 0, and is removed once closed (the
  original keeps the closed shell, drawing nothing).
- A Vanish or Long Door cast while a Constant Displacement runs takes its
  shell: the power's shared flag is set (it writes no closing state at its
  end), the vanish makes no shell and no creation draws, its phased body is
  lent, and the shell runs through the vanish's phases and goes back to the
  constant.
- The fade half (0x400): each body part draws with a blended copy of its
  material at the shown amount that writes its own depth (the depth copy
  steps aside on those frames); a part without a material of the original's
  kind shows above one half.
- The body's shadow: while the staging entries' fade bit is set the body
  casts dithered into the cube shadows at the node fade, one less the shown
  amount of each fade-half frame (`exa_render::cube_shadows::ShadowDither`,
  the original's `fract(3.555713358·x + 0.309269245·y) < Fade` discard), its
  faces dirtied when the dither changes.

## Capabilities

### Modified Capabilities

- `thaumaturgy`: the constant's shell, the vanish's reuse, the blended body
  and the body's shadow.
- `rendering`: the fire textures and the dithered cube casters.

## Impact

- `exa_world::flame`, `exa_render::cube_shadows` (+ `cube_shadow.wesl`),
  `exa_game::thaum::{vanish_live, cast, live}`.
- Evidence: REA (Ghidra) pseudo-code and assembly of `CreateFire` and the
  small fire builder (the `_FirePrt02` and `_FirePrt01` references), the
  shell's constructor, render callback and uniform setter, the shell
  creator, the constant's start and control handler, the vanish start
  (the constant lookup by id 0x20603), its destroy, its two frame callbacks
  and segment pass, the stage/reveal/show/hide calls and the per-node show
  call, and the cube caster pass (the fade bit and the node fade).

## Out of scope

- The sun shadow's dither (Bevy's sun pass still drops a hidden body), the
  shell's stamp on the body's physics clock (the effects' clock stands in),
  per-sub-object blend stages.
