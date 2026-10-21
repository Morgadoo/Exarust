# Proposal

## Why

The force and environment effect nodes of the forces2 group (the rain,
the portal's spray, the flame node, the force domes, the Force Wave, the
vanish, the entities the AI sees, the GI emissives and the force
geometries) were reimplemented on a branch largely from the decompiler's
pseudo-code and partly by guess: the rain box was twice its size and its
streak took a frame length for the camera's tilt, the dome's strike and
tick were confused (a new ripple jumped to the origin), the Barrier's
radius was its cast sound's height, the wave's ring stood 300 cm out with
an invented front mesh, the vanish showed and hid the wrong way round,
the AI's face word used the effect's aim, and a second drawing path drew
shells at centimetre scale on a metre scene. This change reviews every
rule against the instruction listing and keeps one faithful set.

## What Changes

- Rain: a 300 cm half-extent box 450 cm ahead of the eye, the gust walk
  scaled by the capped update figure, heads moved and tails streaked by
  the camera's tilt, wraps on the effect's own generator words, the
  environment-tinted colour, half as many drops drawn as vertices counted.
- Portal spray: previous and current positions (an outward step of 0.0015
  of the offset), the colour word copied, the ring flag cleared on a wrap.
- Force domes: the ripple strike (summed points, capped at 5, nearest
  target), the ripple tick (swap removal, the step's 0.05 in double), the
  cast sound's place, the frame's level of detail, shader points, share
  blend, fade and hum; the Barrier's damped height, focus drain, 800 ms
  grow window, drawn radius, pose snapshots, cap frame and keep-out; the
  Bubble's grow, break and shrink and its two keep-outs; the character
  contact and balance drain; double-sided meshes.
- Force Wave: the 32-chunk ring 80 cm out with twelve-slot trails, the
  charging and thrown ticks over thaum's existing wave rules, lives and
  bounds.
- Vanish: hide, show, start, phase hook and finish.
- AI sight: the blast, wave and barricade assessments and the steps away.
- GI emissives: extents, colours and phase rules of the seven lights.
- Effect geometries: the per-pass texture rows (level of detail), fades
  and program uniforms; the flame node's light.
- The unwired `force_draw` module is removed.

## Capabilities

### Modified Capabilities

- `effect-nodes`: the force and environment effect nodes.
- `thaumaturgy`: the barrier ripple step's precision, the barrier update
  fraction's direction and the barrier's cast volume.

## Impact

- `exa_game::effects::{rain, portal_effect, fire_node, force_dome,
  force_wave, vanish, danger, emissive, effect_geometry}`, `exa_game::thaum`
  (`step_ripple`, `update_barrier_position`, `BARRIER_CAST_VOLUME`);
  `docs/THAUMATURGY.md`, `docs/RENDERING.md`, the `exa_game` README.
- Evidence: the forces2 evidence pack (pseudo-code, instruction listing,
  constants) and REA/Ghidra reads of `Exanima.exe` (the 2-D length helper,
  the component multiply, the teleporter colour after the force texture's
  name, the pass count word).
- Out of scope: the power classes that create these effects (another
  lane), the blast and torrent geometries and emissives, the GL programs,
  frame-buffer passes and GI injection, sounds, registering the AI
  entities, and drawing any of these effects in the running game.

## Phase

Phase 5, visual and audio fidelity: thaumaturgy effects (force waves,
barriers) and environment effects; the AI sight serves Phase 6.
