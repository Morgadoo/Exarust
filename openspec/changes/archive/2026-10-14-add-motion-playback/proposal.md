# Proposal

## Why

Characters are the core of Exanima. Phase 3 of `docs/PLAN.md` (milestone M3)
starts with the skeleton and the motion clips the physics body will follow.

## What Changes

- Decode `humanoid.rsq`: the 18-bone skeleton and the motion set (48 actions
  × 32 movesets with parent fallback, keyed poses with root positions).
- Sample clips onto body-part hierarchies (`exa_anim`).
- `exarust character [moveset/action]` shows the human body playing a clip;
  `exa-tool rsq` summarises the file.

Evidence: the `.rsq` reader in `Exanima.exe` (REA); a byte-exact parse of
the shipped file; screenshots of the walk cycle.

Out of scope: seam stitching between body parts (skin builder), apparel,
physics-driven bodies, blending between clips, root-motion locomotion.

## Capabilities

### New Capabilities
- `motion-playback`: decoding the skeleton and motion clips and posing a character with them.

### Modified Capabilities

## Impact

`crates/exa_formats` (`rsq`), `crates/exa_anim`, `crates/exa_tool`,
`crates/exarust`, `docs/formats/RSQ.md`, `docs/PLAN.md`.
