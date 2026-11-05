# Tasks

- [x] Trace in REA, against the assembly, every writer of the character position, the frame build, the per-tick grid snap and the feet routine's balance reset.
- [x] Remove the capsule, its handles, registration, release, kinematic targets and sweep statistics from `exa_physics`; `CharacterBody::step` moves a character to its follow point and onto its ground, or walks one without muscles on the level; tests.
- [x] `Ragdoll::character_ground`; the carry stage hands the character its ground; the perched exception and `Bearing::carried` removed; tests updated to the original's rule.
- [x] The level teardown, the scene body despawn and the revive no longer handle capsules; the look-ahead's no-grid clearance is a constant.
- [x] Crate READMEs, `docs/formats/RSQ.md`, `docs/PLAYER.md` and the motile-dynamics map; no docs/PLAN.md or COMPONENT-MAP change. No format page changes and no corpus check applies (no data is read).
- [x] GPU check in one grid: walking in exanima01 (start room and the stair room), the sewer and the arena; the sewer's fixed tick before and after.
- [x] fmt, clippy -D warnings and tests for `exa_physics`, `exa_world`, `exa_game`, `exa_ai` and `exarust`, and the play build, on the worker.
