# Proposal

## Why

The character generator (the builder of a body from its record and race:
apparel layering, the skin job's look, hair, the rebuild, spawn and equip
rules) had only its material set-up and outfit dressing in Exarust. A
porting lane added its remaining routines, but many were ported from the
pseudo-code or invented; this change records them as the review corrected
them against the instructions.

## What Changes

- `exa_character::layers`: the slot table read from the executable's
  records (own number, parent, flag word, children); the builder's
  per-garment coverage pass (occupancy bytes against the worn set's
  union), the slot assembler (groups, pruning under the parent-slot node,
  the same-bits fallback, the walk, the first group chosen under the
  select masks), the cull bits and the body mask from the union's
  innermost byte.
- `exa_character::look` and `texture_job`: the normal layers on physique
  maps 1 to 4 and the men's extra map, no layers without a controller or
  for a woman of a men-only race, the replaced zero look seed, the beard
  colour, the job reset, the hair codes and the head coverage bits.
- `exa_assets::hair`: the hair material pass and picker with their
  mixed-precision steps.
- `exa_character::runtime`, `rebuild`, `spawn`, `background`, `factory`:
  the body passes (held marks, slot chain and numbering, re-framing, morph
  refresh, frame save and restore, state copy, detach, dynamics
  build/split/rebuild, clone, merge order, overlays, saved poses), the
  rebuild steps, the equip pass, the spawn motions and heights and the
  background runner.
- `exa_assets::merge`: the occlusion alpha.
- The running game: placed characters' hands come from the equip pass
  (resolved by hand index, the torch section read from the object
  database), their hair follows the head coverage bits; a saved-pose
  system that wrote world frames as local transforms is removed.

## Capabilities

### New Capabilities

- `character-generator`: the generator's layering, look, hair, body,
  rebuild, equip and spawn rules.

### Modified Capabilities

## Impact

- `exa_character` (`layers`, `look`, `texture_job`, `hair`, `runtime`,
  `rebuild`, `spawn`, `background`, `factory`, `lib`), `exa_assets`
  (`hair`, `merge`), `exa_formats` (`objdb` torch section and slot words,
  `races` look word, `rfc` reciprocal square root), `exa_world` (hand torch
  flag, worn head word, placement pose), `exarust` (viewer wiring).
- Docs: `docs/formats/OTHER.md` (the coverage pass), `docs/RENDERING.md`
  (skin job, hair), the `exa_character`, `exa_assets`, `exa_formats` and
  `exa_game` READMEs.
- Evidence: the generator's instructions in the chargen evidence pack
  (slot table, coverage pass, assembler, merge and cull passes, skin job,
  hair material and builder, rebuild, spawns, equip pass, body passes) and
  the executable's own data and helpers for the slot table, the picker,
  its colour helpers and the vector helpers (normalise, dot, cross,
  place, relative frame, distance). Every floating-point step of the hair
  colour was checked against an instruction-level emulation; the shipped
  hair material's mark word was read from `CharExt.rpk`.

## Out of scope

- Drawing the merged single mesh, its cull bits and occlusion alpha; the
  skin texture composite (tone colour, beard, normal blend); the
  `HarHPC1` hair texture set; hair cloth and smoothing; the feral apparel
  overlay of the texture-only rebuild.
- Wiring the rebuild, saved poses, spawn heights and off-hand items into
  the running game.
- The engine's shape builder, skin builder and scene registration the
  generator calls.

## Phase

Phase 3, bodies (the character factory that builds a body from its
record), with the look work of Phase 5 (visual and audio fidelity).
